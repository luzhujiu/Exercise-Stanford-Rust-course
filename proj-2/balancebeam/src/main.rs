mod request;
mod response;

#[macro_use]
extern crate error_chain;

use clap::Clap;
use rand::{Rng, SeedableRng};
//use tokio::{net::TcpListener, net::TcpStream, stream::StreamExt};
use tokio::{net::TcpListener, net::TcpStream};
//use std::sync::Arc;
//use rand::{thread_rng, seq::SliceRandom};
use std::time::Duration;
//use tokio::time::Interval;
use tokio::sync::watch::{Sender, Receiver};

use tokio::runtime::Runtime;

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
#[derive(Debug, Clone)]
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    /// Addresses of servers that we are proxying to
    upstream_addresses: Vec<String>,
}

type Report = Option<Vec<usize>>;

#[tokio::main]
async fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    
    pretty_env_logger::init();

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

    let mut runtime = Runtime::new().expect("failed to start new Runtime");
    let (tx, rx) = tokio::sync::watch::channel(None); 
     
    //do health check
    let state_clone = state.clone();
    let handle = runtime.spawn(async move {
        health_check(&state_clone, tx).await;
    });

    while let Ok((stream, _)) = listener.accept().await {    
        let rx1 = rx.clone(); 
        let state = state.clone();
        runtime.spawn(async move {
            handle_connection(stream, &state, rx1).await;
        });
    }    
    
    runtime.shutdown_background();
    log::info!("shut down.")
}

fn get_report(mut rx: &Receiver<Report>) -> Report {
    (*rx).borrow().clone()
} 

async fn connect_to_upstream(state: &ProxyState, rx: Receiver<Report>) -> Result<TcpStream> {
    let mut rng = rand::rngs::StdRng::from_entropy();
    let mut failed_server_self = vec![];
    loop {
        let report = get_report(&rx);

        let failed_server =
            if let Some(report) = report { 
                failed_server_self = vec![];
                log::info!("report received. {:?}", report);
                report
            } else {
                log::info!("report not received.");
                failed_server_self.clone()
            };

        if failed_server.len() == state.upstream_addresses.len() {
            break;
        }
        let idx = rng.gen_range(0, state.upstream_addresses.len());
        if failed_server.contains(&idx) {
            continue;
        }
        match TcpStream::connect(&state.upstream_addresses[idx]).await {
            Ok(stream) => {
                return Ok(stream);
            },
            Err(e) => {
                failed_server_self.push(idx);
                log::info!("Server-down is detected within connect_to_upstream = {}", idx);
                log::info!("Server-down is not received. last report = {:?}", failed_server);
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

async fn handle_connection(mut client_conn: TcpStream, state: &ProxyState, receiver: Receiver<Report>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);

    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(&state, receiver).await {
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

//Health check -- milestone 4
async fn health_check(state: &ProxyState, sender: Sender<Report>) {
    let seconds = state.active_health_check_interval;
    let mut interval = tokio::time::interval(Duration::from_secs(seconds as u64));
    let path = &state.active_health_check_path;
    log::info!("Health check start. -> interval {} seconds", seconds);
    
    loop {
        let mut failed_servers: Vec<usize> = vec![];
        for (i, ip) in state.upstream_addresses.iter().enumerate() {                             
            let response = health_check_upstream(&ip, &path).await;
            if response.is_err() {
                failed_servers.push(i);
            }
        }
        log::info!("Health check send report -> failed_servers = {:?}", failed_servers);
        sender.send(Some(failed_servers)).unwrap();
        interval.tick().await;
    }    
}

async fn health_check_upstream(upstream: &str, path: &str) -> Result<()>{  
    log::info!("Health Check Start for {} : {}", upstream, path);  
    if let Ok(mut upstream_conn) = TcpStream::connect(upstream).await {
        let request: http::Request<Vec<u8>> = http::Request::builder()
            .method(http::Method::GET)
            .uri(path)
            .header("Host", upstream)
            .body(Vec::new())
            .unwrap();

        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::info!("Health Check Not PASS {} -> Failed to send request to upstream", upstream);
            return Err("Failed to send request to upstream.".into());
        };
        
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await {
            Ok(response) => response,
            Err(error) => {
                log::info!("Health Check NOT PASS {} -> Error reading response from server", upstream);
                return Err("Error reading response from upstream.".into());
            }
        };

        if response.status() == http::StatusCode::OK {
            log::info!("Health Check PASS. {} is running.", upstream);
            return Ok(())
        } else {
            log::info!("Health Check NOT PASS {} -> Response status is not Ok {}", upstream, response.status().as_u16());
            return Err("Response status is not ok.".into());
        }
    } else {
        log::info!("Health Check NOT PASS {} -> connection error", upstream);
        return Err("Connection Error".into());
    }
}