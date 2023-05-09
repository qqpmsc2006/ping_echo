use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::timeout;

use super::magic_head::MagicHead;
use super::request_manager::RequestManager;

pub struct PongReceiver {
    socket: Arc<UdpSocket>,
    requests: Arc<RequestManager>,
    stop: Arc<Mutex<bool>>,
}

impl PongReceiver {
    pub fn new(socket: Arc<UdpSocket>, requests: Arc<RequestManager>) -> Self {
        Self {
            socket,
            requests,
            stop: Arc::new(Mutex::new(false)),
        }
    }

    pub fn stop(&self) {
        let mut stop = self.stop.lock().unwrap();
        *stop = true;
    }

    pub async fn run(&self) {
        let head = MagicHead::new();
        let mut buf: [u8; 32] = [0u8; 32];

        'outer: loop {
            match timeout(Duration::from_secs(1), self.socket.recv_from(&mut buf)).await {
                Ok(r) => match r {
                    Ok((len, peer_addr)) => {
                        self.requests.notify(peer_addr, head == buf[0..len]);
                    }
                    Err(_) => {
                        println!("local socket invalid");
                        exit(0);
                    }
                },
                Err(_) => {
                    if *self.stop.lock().unwrap() {
                        break 'outer;
                    }
                }
            }
        }
    }
}

impl Clone for PongReceiver {
    fn clone(&self) -> Self {
        Self {
            socket: Arc::clone(&self.socket),
            requests: Arc::clone(&self.requests),
            stop: Arc::clone(&self.stop),
        }
    }
}
