//extern crate reqwest;
extern crate select;
#[macro_use]
extern crate error_chain;

use crossbeam_channel;
use std::{thread, time};
use std::default::Default;
use std::time::Instant;
use select::document::Document;
use select::predicate::Name;

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(std::io::Error);
    }
}

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default + Clone,
{
    let mut output_vec: Vec<U> = vec![Default::default(); input_vec.len()];
    let (sender, receiver) = crossbeam_channel::unbounded();
    let (sender2, receiver2) = crossbeam_channel::unbounded();
    
    let mut threads = Vec::new();

    for _ in 0..num_threads {
        let receiver = receiver.clone();
        let sender2 = sender2.clone();
        threads.push(thread::spawn(move || {
            while let Ok((i, elem)) = receiver.recv() {
                let output = f(elem);
                sender2
                    .send((i, output))
                    .expect("Tried writing to channel, but there are no receivers!");
                
            }
        }))
    }

    for (i, elem) in input_vec.into_iter().enumerate() {
        sender
            .send((i, elem))
            .expect("Tried writing to channel, but there are no receivers!");
    }

    drop(sender);

    for thread in threads {
        thread.join().expect("Panic occurred in thread");
    }

    for _ in 0..output_vec.len() {
        let (i, output) = receiver2.recv().expect("should receive output");
        output_vec[i] = output;
    }
    output_vec
}

fn main() -> Result<()> {
    let body = reqwest::blocking::get("https://en.wikipedia.org/wiki/Multithreading_(computer_architecture)")?
    .text()?;
    
    let links = Document::from_read(body.as_bytes())?
        .find(Name("a"))
        .filter_map(|n| {
            if let Some(link_str) = n.attr("href") {
                if link_str.starts_with("/wiki/") {
                    Some(format!("{}/{}", "https://en.wikipedia.org",
                        &link_str[1..]))
                } else {
                    None
                }
            } else {
                None
            }
        }).collect::<Vec<String>>();

    let start = Instant::now();    
    
    let outputs = parallel_map(links, 10, |link| {
        let body = reqwest::blocking::get(&link).expect("").text().expect("");
        (link, body.len())
    });
    
    let max = outputs.iter().max_by(|x,y| x.1.cmp(&y.1)).unwrap();
    println!("max = {:?}", max);
    println!("Total execution time: {:?}", start.elapsed());
    Ok(())
}
