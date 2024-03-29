use std::sync::{
    mpsc::{self, channel, Sender},
    Arc, Mutex,
};

use crate::http_server::thread_pool::worker::{Message, Worker};
use tracing::{error, info};

pub enum ThreadPoolError {
    MessageSendError(mpsc::SendError<Message>),
}

/// Struct that represents a thread pool that spawns a specified number of worker threads and allows to process connections concurrently.
/// Each idle thread in the pool is available to handle a task.
/// When a thread is done processing its task, it is returned to the pool of idle threads, ready to handle a new task.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Message>,
}

impl ThreadPool {
    /// Creates a new ThreadPool with a given size.
    /// The size is the number of threads in the pool.
    /// If the size is zero or a negative number, the `new` function will panic.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    /// Receives a closure and assigns it to a thread in the pool to run.
    pub fn execute<F>(&self, closure: F) -> Result<(), ThreadPoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(closure);

        self.sender
            .send(Message::NewJob(job))
            .map_err(ThreadPoolError::MessageSendError)?;

        Ok(())
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        info!("Sending terminate message to all workers.");

        for _ in &self.workers {
            if self.sender.send(Message::Terminate).is_err() {
                error!("An error occurred while attempting to drop the thread pool.");
            };
        }

        info!("Shutting down all workers.");

        for worker in &mut self.workers {
            info!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                if thread.join().is_err() {
                    error!("An error occurred while attempting to join a thread pool thread.");
                };
            }
        }
    }
}
