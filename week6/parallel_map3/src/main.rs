use crossbeam_channel;
use std::{thread, time};
use std::default::Default;
use std::time::Instant;
use crate::threadpool::ThreadPool;

mod threadpool;

fn parallel_map<T, U, F>(input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default + Clone,
{
    let mut pool = ThreadPool::new(num_threads);
    let mut output_vec: Vec<U> = vec![Default::default(); input_vec.len()];
    
    let (tx, rx) = crossbeam_channel::unbounded();
    
    for (i, input) in input_vec.into_iter().enumerate() {
        let tx = tx.clone();
        pool.execute(move || {
            let output = f(input);
            tx.send((i, output)).expect("Tried writing to channel, but there are no receivers!");
        })
    }

    pool.drop();
    pool.join();

    for _ in 0..output_vec.len() {
        let (i, output) = rx.recv().expect("should receive output");
        output_vec[i] = output;
    }

    output_vec
}

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
