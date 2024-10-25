//! Thread pool that joins all thread when dropped.

// NOTE: Crossbeam channels are MPMC, which means that you don't need to wrap the receiver in
// Arc<Mutex<..>>. Just clone the receiver and give it to each worker thread.
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};

struct Job(Box<dyn FnOnce() + Send + 'static>);

#[derive(Debug)]
struct Worker {
    _id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for Worker {
    /// When dropped, the thread's `JoinHandle` must be `join`ed.  If the worker panics, then this
    /// function should panic too.
    ///
    /// NOTE: The thread is detached if not `join`ed explicitly.
    fn drop(&mut self) {
        // todo!()
        if let Some(thread) = self.thread.take() {
            if let Err(e) = thread.join() {
                panic!("Worker thread panicked: {:?}", e);
            }
        }
    }
}

/// Internal data structure for tracking the current job status. This is shared by worker closures
/// via `Arc` so that the workers can report to the pool that it started/finished a job.
#[derive(Debug, Default)]
struct ThreadPoolInner {
    job_count: Mutex<usize>,
    empty_condvar: Condvar,
}

impl ThreadPoolInner {
    /// Increment the job count.
    fn start_job(&self) {
        *self.job_count.lock().unwrap() += 1;
        // 离开作用域时自动drop
        // drop(self.job_count.lock().unwrap());
    }

    /// Decrement the job count.
    fn finish_job(&self) {
        let mut job_count = self.job_count.lock().unwrap();
        *job_count -= 1;
        if *job_count == 0 {
            self.empty_condvar.notify_all();
        }
    }

    /// Wait until the job count becomes 0.
    ///
    /// NOTE: We can optimize this function by adding another field to `ThreadPoolInner`, but let's
    /// not care about that in this homework.
    fn wait_empty(&self) {
        let mut job_count = self.job_count.lock().unwrap();
        while *job_count != 0 {
            job_count = self.empty_condvar.wait(job_count).unwrap();
        }
    }
}

/// Thread pool.
#[derive(Debug)]
pub struct ThreadPool {
    _workers: Vec<Worker>,
    job_sender: Option<Sender<Job>>,
    pool_inner: Arc<ThreadPoolInner>,
}

impl ThreadPool {
    /// Create a new ThreadPool with `size` threads.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        // todo!()
        let mut workers = Vec::with_capacity(size);
        let (job_sender, receiver) = crossbeam_channel::unbounded::<Job>();

        let pool_inner = Arc::new(ThreadPoolInner::default());

        let receiver = Arc::new(receiver);

        for id in 0..size {
            let receiver = Arc::clone(&receiver);
            let pool_inner = Arc::clone(&pool_inner);
            workers.push(Worker {
                _id: id,
                thread: Some(thread::spawn(move || loop {
                    match receiver.recv() {
                        Ok(Job(job)) => {
                            job();
                        }
                        Err(_) => break,
                    }
                })),
            });
        }
        ThreadPool {
            _workers: workers,
            job_sender: Some(job_sender),
            pool_inner,
        }
    }

    /// Execute a new job in the thread pool.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.pool_inner.start_job();
        self.job_sender.as_ref().unwrap().send(Job(job)).unwrap();
        self.pool_inner.finish_job();
    }

    /// Block the current thread until all jobs in the pool have been executed.
    ///
    /// NOTE: This method has nothing to do with `JoinHandle::join`.
    pub fn join(&self) {
        // todo!()
        self.pool_inner.wait_empty();
    }
}

impl Drop for ThreadPool {
    /// When dropped, all worker threads' `JoinHandle` must be `join`ed. If the thread panicked,
    /// then this function should panic too.
    fn drop(&mut self) {
        // todo!()
        self.pool_inner.wait_empty();
        drop(self.job_sender.take());
        for worker in self._workers.iter_mut() {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
