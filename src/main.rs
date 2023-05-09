use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::timeout;

struct MagicHead([u8; 4]);

impl MagicHead {
    fn new() -> Self {
        Self(*b"j-wy")
    }
}

impl Deref for MagicHead {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<[u8]> for MagicHead {
    fn eq(&self, other: &[u8]) -> bool {
        &self.0 == other
    }
}

enum PingResult {
    Ok,
    Failed,
    Timeout,
}

struct Ping {
    socket: Arc<UdpSocket>,
    requests: Arc<Requests>,
}

impl Ping {
    async fn new() -> Self {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        let requests = Arc::new(Requests::new());

        {
            let rs = Arc::clone(&socket);
            let rr = Arc::clone(&requests);
            tokio::task::spawn(async move {
                let pr = PongReceiver::new(rs, rr);
                pr.run().await
            });
        }

        Self {
            socket: Arc::clone(&socket),
            requests: Arc::clone(&requests),
        }
    }

    async fn send(&self, target: SocketAddr) {
        let mut timeout_count = 0;
        let mut success_count = 0;
        let mut elapsed = Duration::new(0, 0);

        for _ in 0..10 {
            let begin = std::time::Instant::now();
            match self.do_send_once(&target).await {
                PingResult::Ok => {
                    let end = std::time::Instant::now();
                    elapsed += end.duration_since(begin);
                    success_count += 1;
                }
                PingResult::Failed => {
                    println!("{} failed", target.to_string());
                    return;
                }
                PingResult::Timeout => {
                    timeout_count += 1;
                    println!("{} timeout {} ...", target.to_string(), timeout_count);
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

    async fn do_send_once(&self, target: &SocketAddr) -> PingResult {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.requests.register(target.clone(), tx);

        let head = MagicHead::new();
        match self.socket.send_to(&head, target).await {
            Ok(4) => {}
            _ => {
                self.requests.unregister(&target);
                return PingResult::Failed;
            }
        };

        let res = match timeout(Duration::from_secs(3), rx).await {
            Ok(r) => match r {
                Ok(true) => PingResult::Ok,
                _ => PingResult::Failed,
            },
            _ => PingResult::Timeout,
        };

        self.requests.unregister(&target);

        res
    }
}

impl Clone for Ping {
    fn clone(&self) -> Self {
        Self {
            socket: Arc::clone(&self.socket),
            requests: Arc::clone(&self.requests),
        }
    }
}

struct PongReceiver {
    socket: Arc<UdpSocket>,
    requests: Arc<Requests>,
}

impl PongReceiver {
    fn new(socket: Arc<UdpSocket>, requests: Arc<Requests>) -> Self {
        Self { socket, requests }
    }

    async fn run(&self) {
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

struct Requests {
    requests: Mutex<HashMap<std::net::SocketAddr, tokio::sync::oneshot::Sender<bool>>>,
}

impl Requests {
    fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
        }
    }

    fn register(&self, addr: std::net::SocketAddr, tx: tokio::sync::oneshot::Sender<bool>) {
        let mut r = self.requests.lock().unwrap();
        r.insert(addr, tx);
    }

    fn unregister(&self, addr: &std::net::SocketAddr) {
        let mut r = self.requests.lock().unwrap();
        r.remove(addr);
    }

    fn notify(&self, addr: std::net::SocketAddr, res: bool) {
        let mut r = self.requests.lock().unwrap();
        if let Some(tx) = r.remove(&addr) {
            let _ = tx.send(res);
        }
    }

    fn is_empty(&self) -> bool {
        let r = self.requests.lock().unwrap();
        r.is_empty()
    }
}

#[tokio::main]
async fn main() {
    let ips = [];

    let mut js = tokio::task::JoinSet::new();

    for ip in ips {
        let ping = Ping::new().await;
        for i in 0..=8 {
            let ping = ping.clone();
            js.spawn(async move {
                ping.send(std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                    std::net::Ipv4Addr::from_str(ip).unwrap(),
                    10000 + i,
                )))
                .await
            });
        }
    }

    while let Some(_) = js.join_next().await {}
}
