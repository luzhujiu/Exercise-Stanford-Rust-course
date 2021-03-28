use crossbeam_channel;
use std::{thread, time};
use std::time::Instant;
use crate::threadpool::ThreadPool;
use crossbeam_channel::Receiver;

mod threadpool;

fn reduce<T,F>(pool: &ThreadPool, rx: Receiver<T>, f: F) -> T
where
    F: FnOnce(T, T) -> T + Send + Copy + 'static,
    T: Send + 'static + std::fmt::Display + Sync + Copy,
{
    if rx.len() == 1 {
        let x = rx.recv().expect("Tried receive from channel");
        return x;
    }

    let (tx2, rx2) = crossbeam_channel::unbounded::<T>();
    loop {
        if let Ok(x) = rx.recv() {
            if let Ok(y) = rx.recv() {
                let tx = tx2.clone();
                pool.execute(move || {
                    let output = f(x, y);
                    tx.send(output).expect("Tried writing to channel, but there are no receivers!");
                });
            } else {
                let tx = tx2.clone();
                pool.execute(move || {
                    tx.send(x).expect("Tried writing to channel, but there are no receivers!");
                });
                break;
            }
        } else { 
            break;
        }
    }

    drop(tx2);
    return reduce(pool, rx2, f);
}

fn parallel_reduce<T, F>(input_vec: Vec<T>, num_threads: usize, f: F)  -> T
where
    F: FnOnce(T, T) -> T + Send + Copy + 'static,
    T: Send + 'static + std::fmt::Display + Sync + Copy,
{
    let mut pool = ThreadPool::new(num_threads);
    
    let (tx, rx) = crossbeam_channel::unbounded();
    
    for elem in input_vec {
        tx.send(elem).expect("Tried writing to channel, but there are no receivers!");
    }

    drop(tx);

    let ans = reduce(&pool, rx, f);

    pool.drop();
    pool.join();

    return ans;
}

fn main() {
    let start = Instant::now();
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let ans = parallel_reduce(v, 10, |x, y| {
        thread::sleep(time::Duration::from_millis(500));
        x + y
    });    
    println!("answer = {}", ans);
    println!("Total execution time: {:?}", start.elapsed());
}
