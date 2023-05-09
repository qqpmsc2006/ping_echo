use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::timeout;

use super::magic_head::MagicHead;
use super::request_manager::RequestManager;

pub struct PongReceiver {
    socket: Arc<UdpSocket>,
    requests: Arc<RequestManager>,
}

impl PongReceiver {
    pub fn new(socket: Arc<UdpSocket>, requests: Arc<RequestManager>) -> Self {
        Self { socket, requests }
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
                    Err(_) => break 'outer,
                },
                Err(_) => {
                    if self.requests.is_empty() {
                        break 'outer;
                    }
                }
            }
        }
    }
}
