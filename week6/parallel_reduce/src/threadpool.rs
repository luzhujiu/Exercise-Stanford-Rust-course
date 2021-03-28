use crossbeam_channel;
use crossbeam_channel::Sender;
use std::thread;
use std::thread::JoinHandle;

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

type Thunk<'a> = Box<dyn FnBox + Send + 'a>;

pub struct ThreadPool {
    tx: Option<Sender<Thunk<'static>>>,
    threads: Option<Vec<JoinHandle<()>>>
}

impl ThreadPool
{
    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx.as_ref().unwrap().clone()
            .send(Box::new(job))
            .expect("ThreadPool::execute unable to send job into queue.");
    }

    pub fn new(num_threads: usize) -> ThreadPool {
        let (tx, rx) = crossbeam_channel::unbounded::<Thunk<'static>>();
        let mut threads = vec![];
        for _ in 0..num_threads {
            let rx = rx.clone();
            threads.push(thread::spawn(move || {
                loop {
                    if let Ok(job) = rx.recv() {
                        job.call_box();
                    } else {
                        break;
                    }
                }
            }))
        }

        ThreadPool {
            tx: Some(tx),
            threads: Some(threads)
        }
    }

    pub fn join(&mut self) {
        let threads = self.threads.take().unwrap();
        for thread in threads{
            thread.join().expect("Panic occurred in thread");
        }
    }

    pub fn drop(&mut self) {
        let tx = self.tx.take().unwrap();
        drop(tx);
    }
}
