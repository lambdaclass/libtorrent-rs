use std::{
    sync::{mpsc::Receiver, Arc, Mutex},
    thread,
};

use logger::logger_sender::LoggerSender;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub enum Message {
    NewJob(Job),
    Terminate,
}
/// Struct responsible for sending code from the ThreadPool to a Thread.
pub struct Worker {
    // TODO: solve public attributes
    pub id: usize,
    pub thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Returns a new Worker instance that holds the `id` and a thread spawned with an empty closure.
    pub fn new(
        id: usize,
        receiver: Arc<Mutex<Receiver<Message>>>,
        logger_sender: LoggerSender,
    ) -> Worker {
        let thread = thread::spawn(move || loop {
            while let Ok(message) = receiver.lock().unwrap().recv() {
                // unwrap is safe because we are the only one using the Receiver.
                match message {
                    Message::NewJob(job) => {
                        logger_sender.info(&format!("Worker {} got a job; executing.", id));
                        job();
                    }
                    Message::Terminate => {
                        logger_sender.info(&format!("Worker {} was told to terminate.", id));
                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
