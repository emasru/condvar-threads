use std::{sync::{Arc, Mutex}, thread::{self, JoinHandle}};

struct Worker {
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Worker {
    fn post<F>(&mut self, f: F)
    where
        F: Fn() + Send + 'static, // Ensure F is Send and 'static
    {
        // Join the previous handle, or wait until it is finished
        let handle = Arc::clone(&self.handle);
        let mut guard = handle.lock().unwrap();
        if let Some(handle) = guard.take() {
            handle.join().unwrap();
        }

        // Spawn a new thread to execute the closure
        let handle = thread::spawn(move || {
            f();
        });

        // Update the worker handle
        self.handle = Arc::new(Mutex::new(Some(handle)));
    }
}

struct Workers {
    workers: Vec<Worker>,
    // Use an RR scheduling system
    next_thread: Arc<Mutex<usize>>
}

impl Workers {
    fn new(max_threads: usize) -> Workers {
        let mut workers: Vec::<Worker> = Vec::new();
        let next_thread = Arc::new(Mutex::new(0));
        
        for _ in 0..max_threads {
            workers.push(Worker { handle: Arc::new(Mutex::new(None)) })
        }

        /*
        let watch_thread = thread::spawn(move || {
            // Something to watch .-.
        });
        */

        Workers { workers, next_thread }
    }

    fn post<F>(&mut self, f: F)
    where
        F: Fn() + Send + 'static, 
    {
        // println!("current number of workers is {}", self.workers.len());
        
        // Pick the next available thread
        let mut next_thread = self.next_thread.lock().unwrap();
        // println!("next thread index to be executed is {}", next_thread);
        let worker = &mut self.workers[*next_thread];
        worker.post(f);

        // Increments or wraps around
        if self.workers.len()-1 == 0 {
            *next_thread = 0;
        } else if *next_thread < self.workers.len()-1 {
            *next_thread += 1;
        } else {
            *next_thread = 0;
        }

        // println!("NOW next thread index to be executed is {}", next_thread);

        // Lock releases when ref is dropped
    }

    fn join_all (self) {
        for worker in self.workers {
            let mut guard = worker.handle.lock().unwrap();
            if let Some(handle) = guard.take() {
                handle.join().unwrap();
            }
        }
    }
}

fn main() {
    let mut pool_w_threads = Workers::new(4);

    for i in 0..8 {
        pool_w_threads.post(move || {
            println!("Executing job {} with 4 threads", i);
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