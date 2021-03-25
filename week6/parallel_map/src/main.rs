use crossbeam_channel;
use std::{thread, time};

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    let (sender, receiver) = crossbeam_channel::unbounded();
    let (sender2, receiver2) = crossbeam_channel::unbounded();
    
    let mut threads = Vec::new();

    for _ in 0..num_threads {
        let receiver = receiver.clone();
        let sender2 = sender2.clone();
        threads.push(thread::spawn(move || {
            while let Ok(next_num) = receiver.recv() {
                let output = f(next_num);
                sender2
                    .send(output)
                    .expect("Tried writing to channel, but there are no receivers!");
                
            }
        }))
    }

    for elem in input_vec {
        sender
            .send(elem)
            .expect("Tried writing to channel, but there are no receivers!");
    }

    drop(sender);

    for thread in threads {
        thread.join().expect("Panic occurred in thread");
    }

    for _ in 0..output_vec.capacity() {
        let output = receiver2.recv().expect("should receive output");
        output_vec.push(output);
    }

    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
