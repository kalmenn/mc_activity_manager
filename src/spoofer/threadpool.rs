// Totally didn't steal that from https://doc.rust-lang.org/stable/book/ch20-02-multithreaded.html
// I could have used a crate to handle this, but I wanted to get my hands dirty

use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
        Mutex
    },
    thread::{self, JoinHandle}
};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    tx: Option<Sender<Job>>
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (tx, rx) = mpsc::channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&rx)));
        }

        ThreadPool { 
            workers, 
            tx: Some(tx)
        }
    }

    pub fn execute<F>(&self, f: F)
    where F: FnOnce() + Send + 'static {
        let job = Box::new(f);
        self.tx.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Dropping the transmission end makes the worker recieve an error
        drop(self.tx.take());

        // We wait for all of the workers to have finished serving any request
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    thread: Option<JoinHandle<()>>
}

impl Worker {
    fn new(id: usize, rx: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || { loop {
            let message = rx.lock().unwrap().recv();
            match message {
                Ok(job) => { 
                    println!("Running job on worker {id}");
                    job() },
                Err(_) => { break }
            }
        }});

        Worker {thread: Some(thread) }
    }
}