extern crate select;
#[macro_use]
extern crate error_chain;
use std::{thread, time};
use std::sync::{Arc, Mutex, Condvar};
use std::collections::VecDeque;
use std::time::Instant;
use select::document::Document;
use select::predicate::Name;

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(std::io::Error);
    }
}
#[derive(Clone)]
pub struct SemaPlusPlus<T> {
    queue_and_cv: Arc<(Mutex<VecDeque<T>>, Condvar)>,
}

impl<T> SemaPlusPlus<T> {
    pub fn new() -> Self {
        SemaPlusPlus {queue_and_cv: Arc::new((Mutex::new(VecDeque::new()),
            Condvar::new()))}
    }
    
    // Enqueues -- Like semaphore.signal()
    pub fn send(&self, message: T) {
        let (queue_lock, cv) = &*self.queue_and_cv;
        let mut queue = queue_lock.lock().unwrap();
        let queue_was_empty = queue.is_empty();
        queue.push_back(message);
        if queue_was_empty {
            cv.notify_all();
        }
    }
    
    // Dequeues -- Like semaphore.wait()
    pub fn recv(&self) -> T {
        let (queue_lock, cv) = &*self.queue_and_cv;
        // Wait until there is something to dequeue
        let mut queue = cv.wait_while(queue_lock.lock().unwrap(), |queue| {
            queue.is_empty()
        }).unwrap();
        // Should return Some(...) because we waited
        queue.pop_front().unwrap()
    }
}

fn create_chunks<T: Clone> (input_vec: Vec<(usize,T)>, num_threads: usize) -> Vec<Vec<(usize,T)>> {
    let mut output = vec![vec![]; num_threads];
    for i in 0..input_vec.len() {
        let index = i % num_threads;
        output[index].push(input_vec[i].clone());
    }
    return output;
}

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static + Sync + Clone + std::fmt::Debug,
    U: Send + 'static + Default + Clone,
{
    let input_vec = input_vec.into_iter().enumerate().collect::<Vec<_>>();
    let chunks: Vec<Vec<(usize,T)>> = create_chunks(input_vec, num_threads);

    let sem: SemaPlusPlus<Vec<(usize, U)>> = SemaPlusPlus::new();
    let mut handles = Vec::new();

    for i in 0..num_threads {
        let chunks = chunks[i].clone();
        let sem_clone = sem.clone();
        let handle = thread::spawn(move || {
            let output = chunks.into_iter().map(|(k, input)| (k, f(input))).collect::<Vec<_>>();
            sem_clone.send(output)
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Panic occurred in thread");
    }

    let mut output_vec = vec![];
    for _ in 0..num_threads {
        output_vec.append(&mut sem.recv());
    }

    output_vec.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    output_vec.into_iter().map(|(_, elem)| elem).collect::<Vec<U>>()
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

/*
fn main() {
    let start = Instant::now();
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });    
    println!("squares: {:?}", squares);
    println!("Total execution time: {:?}", start.elapsed());
}
*/
