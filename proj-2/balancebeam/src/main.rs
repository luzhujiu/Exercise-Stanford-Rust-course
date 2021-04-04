mod request;
mod response;

#[macro_use]
extern crate error_chain;

use clap::Clap;
//use rand::{Rng, SeedableRng};
use tokio::{net::TcpListener, net::TcpStream, stream::StreamExt, sync::Mutex};
use async_std::channel::{unbounded, Sender, Receiver};
use async_std::task;
use std::sync::Arc;
use std::time::Instant;
use rand::{thread_rng, seq::SliceRandom};

error_chain! {}

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Clap, Debug)]
#[clap(about = "Fun with load balancing")]
struct CmdOptions {
    #[clap(
        short,
        long,
        about = "IP/port to bind to",
        default_value = "0.0.0.0:1100"
    )]
    bind: String,
    #[clap(short, long, about = "Upstream host to forward requests to")]
    upstream: Vec<String>,
    #[clap(
        long,
        about = "Perform active health checks on this interval (in seconds)",
        default_value = "10"
    )]
    active_health_check_interval: usize,
    #[clap(
    long,
    about = "Path to send request to for active health checks",
    default_value = "/"
    )]
    active_health_check_path: String,
    #[clap(
        long,
        about = "Maximum number of requests to accept per IP per minute (0 = unlimited)",
        default_value = "0"
    )]
    max_requests_per_minute: usize,
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
#[derive(Clone)]
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    upstream_addresses: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    
    //pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let mut listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    // Handle incoming connections
    let state = ProxyState {
        upstream_addresses: options.upstream,
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
    };

    //channel to maintain failed upstreams.
    let (sender, receiver) = unbounded::<usize>();
   
    let bsenders: Arc<Mutex<Vec<Sender<usize>>>> = Arc::new(Mutex::new(vec![]));
    let bsenders_clone = Arc::clone(&bsenders);

    //receive failed upstream index.
    tokio::spawn(async move {
        while let Ok(idx) = receiver.recv().await {
            let bsenders = bsenders_clone.lock().await;
            for bsender in &(*bsenders) {
                bsender.send(idx).await;
            }

            println!("send idx = {} -> {:?}", idx, Instant::now());
        }
    });

    while let Some(stream) = listener.next().await {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                let sender = sender.clone();
                let (bsender, breceiver) = unbounded::<usize>();
                {
                    let mut bsenders = bsenders.lock().await;
                    bsenders.push(bsender);
                }
                let failed_streams = Arc::new(Mutex::new(vec![]));
                let failed_streams_clone = Arc::clone(&failed_streams);

                tokio::spawn(async move {
                    tokio::spawn(async move {
                        while let Ok(idx) = breceiver.recv().await {
                            let mut vec = failed_streams_clone.lock().await;
                            vec.push(idx);
                            println!("receive vec = {:?} -> {:?}", vec, Instant::now());
                        }
                    });

                    handle_connection(stream, state, sender, failed_streams).await;
                });   
            }
            Err(e) => {
                log::error!("Connection failed. {:?}", e);
                std::process::exit(1);
            }
        }
    }
}

async fn contains(failed_streams: &Arc<Mutex<Vec<usize>>>, idx: usize) -> bool {
    let vec = failed_streams.lock().await;
    println!("vec = {:?} -> {:?}", vec, Instant::now());
    vec.contains(&idx)
}

async fn connect_to_upstream(state: &ProxyState, sender: Sender<usize>, failed_streams: Arc<Mutex<Vec<usize>>>) -> Result<TcpStream> {
    let mut indecies = (0..state.upstream_addresses.len()).collect::<Vec<usize>>();
    indecies.shuffle(&mut thread_rng());

    for idx in indecies {
        
        if contains(&failed_streams, idx).await {
            continue;
        }

        let upstream_ip = &state.upstream_addresses[idx];
        
        match TcpStream::connect(&upstream_ip).await {
            Ok(stream) => {
                return Ok(stream);
            },
            Err(e) => {
                sender.send(idx).await;
            }
        }
    }

    let errmsg = "All upstreams are dead.";
    log::error!("{}",errmsg);
    return Err(errmsg.into());
}

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("{} <- {}", client_ip, response::format_response_line(&response));
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

async fn handle_connection(mut client_conn: TcpStream, state: ProxyState, 
                sender: Sender<usize>, failed_streams: Arc<Mutex<Vec<usize>>>) {
    
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);

    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(&state, sender, failed_streams).await {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
    };
    let upstream_ip = client_conn.peer_addr().unwrap().ip().to_string();

    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn).await {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };
        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!("Failed to send request to upstream {}: {}", upstream_ip, error);
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}