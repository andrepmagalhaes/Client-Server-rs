use log;
use log4rs;
use rand::Rng;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{error, thread};

struct ThreadPool {
    workers: Vec<Worker>,
    job_queue: Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl ThreadPool {
    fn new(size: usize) -> ThreadPool {
        let job_queue = Arc::new(Mutex::new(Vec::new()));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&job_queue)));
        }

        ThreadPool { workers, job_queue }
    }

    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.job_queue.lock().unwrap().push(job);
    }
}

impl Worker {
    fn new(id: usize, job_queue: Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let job = {
                let mut queue = job_queue.lock().unwrap();
                match queue.pop() {
                    Some(job) => job,
                    None => continue,
                }
            };

            log::info!("Worker {} got a job; executing.", id);
            job();
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

fn handle_response(mut stream: TcpStream, response: &[u8]) {
    stream.write(response).unwrap_or_else(|err| {
        log::error!("Unable to write to stream: {:?}", err);
        return 0;
    });
    stream.flush().unwrap_or_else(|err| {
        log::error!("Unable to flush stream: {:?}", err);
    });
}

fn generate_interval() -> Vec<i32> {
    // generate two random numbers where 10 <= n <= 100
    let mut rng = rand::thread_rng();
    let random_number1 = rng.gen_range(10..=100);
    let random_number2 = rng.gen_range(10..=100);
    let mut interval = vec![random_number1, random_number2];

    interval.sort();

    return interval;
}
// request format
// interval => id,interval
// result => id,result,pi_calc

// response format
// interval => id,[interval],server_recieved_time,server_sent_time
// result => id,server_recieved_time,server_sent_time
fn handle_client(mut stream: TcpStream, recieved_time: String) {
    let mut buffer = [0; 256];
    match stream.read(&mut buffer) {
        Ok(_) => {
            let buffer_str = match std::str::from_utf8(&buffer) {
                Ok(v) => v,
                Err(e) => {
                    let error_message = format!("Invalid UTF-8 sequence: {}", e);
                    log::error!("{}", error_message);
                    handle_response(stream, "Invalid UTF-8 sequence".as_bytes());
                    return;
                }
            };
            let buffer_str = buffer_str.trim_end_matches('\0').trim_end_matches('\n');
            let request_to_vec = buffer_str.split(',').collect::<Vec<&str>>();
            log::info!("Request: {:?}", request_to_vec);

            if request_to_vec.len() <= 1 {
                let error_message = "Invalid request format";
                log::error!("{}", error_message);
                handle_response(stream, error_message.as_bytes());
                return;
            }

            match request_to_vec[1] {
                "interval" => {
                    let interval = generate_interval();
                    let sent_time = chrono::Local::now().to_string();
                    let response = format!(
                        "{}|{:?}|{}|{}",
                        request_to_vec[0], interval, recieved_time, sent_time
                    );
                    log::info!("Sending response: {}", response);
                    handle_response(stream, response.as_bytes());
                }
                "result" => {
                    log::info!("{}: {}", request_to_vec[0], request_to_vec[2]);
                    handle_response(stream, "".as_bytes());
                }
                _ => {
                    let error_message = "Invalid request type";
                    log::error!("{}", error_message);
                    handle_response(stream, error_message.as_bytes());
                }
            }
        }
        Err(e) => {
            let error_message = format!("Unable to read from stream: {}", e);
            log::error!("{}", error_message);
            handle_response(stream, error_message.as_bytes());
        }
    };
}
// interval
// result,
fn main() {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();

    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4); // Set the number of threads here

    for stream in listener.incoming() {
        let recieved_time = chrono::Local::now().to_string();
        match stream {
            Ok(stream) => {
                log::info!(
                    "New connection: {}",
                    stream.peer_addr().unwrap_or_else(|err| {
                        log::error!("Unable to get peer address: {:?}", err);
                        return std::net::SocketAddr::new(
                            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                            0,
                        );
                    }),
                );
                pool.execute(|| handle_client(stream, recieved_time));
            }
            Err(e) => {
                log::error!("Unable to connect: {}", e);
            }
        }
    }
}
