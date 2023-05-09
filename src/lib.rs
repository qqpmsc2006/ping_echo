use std::fmt::Display;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tokio::time::timeout;

mod magic_head;
mod pong_receiver;
mod request_manager;

use magic_head::MagicHead;
use pong_receiver::PongReceiver;
use request_manager::RequestManager;

pub enum PingResult {
    Success,
    SendFailed,
    Invalid,
    Timeout,
}

impl Display for PingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PingResult::Success => f.write_fmt(format_args!("Success")),
            PingResult::SendFailed => f.write_fmt(format_args!("SendFailed")),
            PingResult::Invalid => f.write_fmt(format_args!("Invalid")),
            PingResult::Timeout => f.write_fmt(format_args!("Timeout")),
        }
    }
}

pub struct Client {
    socket: Arc<UdpSocket>,
    requests: Arc<RequestManager>,
    receiver: PongReceiver,
}

impl Client {
    pub async fn new() -> Self {
        let socket: Arc<UdpSocket> = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        let requests = Arc::new(RequestManager::new());

        let rs = Arc::clone(&socket);
        let rr = Arc::clone(&requests);
        let receiver = PongReceiver::new(rs, rr);
        {
            let receiver = receiver.clone();
            tokio::task::spawn(async move { receiver.run().await });
        }

        Self {
            socket: Arc::clone(&socket),
            requests: Arc::clone(&requests),
            receiver,
        }
    }

    pub async fn test_target(&self, target: SocketAddr) {
        let mut timeout_count = 0;
        let mut success_count = 0;
        let mut elapsed = Duration::new(0, 0);

        for _ in 0..10 {
            let begin = Instant::now();
            match self.ping_once(&target).await {
                PingResult::Success => {
                    let end = Instant::now();
                    elapsed += end.duration_since(begin);
                    success_count += 1;
                }
                PingResult::Timeout => {
                    timeout_count += 1;
                }
                res => {
                    println!("{} {}", target, res);
                    return;
                }
            };
        }

        println!(
            "{} done, rtt avg: {} ms, success:{}, timeout: {}",
            target.to_string(),
            elapsed.as_millis() / success_count,
            success_count,
            timeout_count,
        );
    }

    pub async fn ping_once(&self, target: &SocketAddr) -> PingResult {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.requests.register(target.clone(), tx);

        let head = MagicHead::new();
        match self.socket.send_to(&head, target).await {
            Ok(4) => {}
            _ => {
                self.requests.unregister(&target);
                return PingResult::SendFailed;
            }
        };

        let res = match timeout(Duration::from_secs(3), rx).await {
            Ok(r) => match r {
                Ok(true) => PingResult::Success,
                _ => PingResult::Invalid,
            },
            _ => PingResult::Timeout,
        };

        self.requests.unregister(&target);

        res
    }
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            socket: Arc::clone(&self.socket),
            requests: Arc::clone(&self.requests),
            receiver: self.receiver.clone(),
        }
    }
}
