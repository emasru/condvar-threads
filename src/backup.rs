use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::time::Duration;

struct Workers {
    threads: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

enum Message {
    Job(Box<dyn FnOnce() + Send + 'static>),
    DelayedJob(Box<dyn FnOnce() + Send + 'static>, u64),
    Terminate,
}

impl Workers {
    fn new(size: usize) -> Workers {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut threads = Vec::with_capacity(size);

        for _ in 0..size {
            let worker = Worker::new(Arc::clone(&receiver));
            threads.push(worker);
        }

        Workers { threads, sender }
    }

    fn post<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Message::Job(Box::new(f));
        self.sender.send(job).unwrap();
    }

    fn post_timeout<F>(&self, f: F, delay_ms: u64)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Message::DelayedJob(Box::new(f), delay_ms);
        self.sender.send(job).unwrap();
    }

    fn join_all(&mut self) {
        for worker in &mut self.threads {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        self.join_all();
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::Job(job) => {
                    job();
                }
                Message::DelayedJob(job, delay_ms) => {
                    thread::sleep(Duration::from_millis(delay_ms));
                    job();
                }
                Message::Terminate => break,
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}

fn main() {
    let mut pool_w_threads = Workers::new(4);

    for i in 0..8 {
        pool_w_threads.post(move || {
            println!("Executing job {} with threads", i);
        });
    }

    let mut pool_w_event_loop = Workers::new(1);

    for i in 0..8 {
        pool_w_event_loop.post(move || {
            println!("Executing job {} with an event loop", i);
        });
    }

    pool_w_threads.post_timeout(|| {
        println!("Executing delayed job after 2 seconds");
    }, 2000);

    pool_w_threads.join_all();
    pool_w_event_loop.join_all();
}