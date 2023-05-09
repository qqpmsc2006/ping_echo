use std::{
    fmt::Display,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
    str::FromStr,
};

use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration, Instant};

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

struct PingClient {
    socket: UdpSocket,
    remote_addr: SocketAddr,
}

impl PingClient {
    async fn new(socket: UdpSocket, remote_addr: SocketAddr) -> Self {
        Self {
            socket,
            remote_addr,
        }
    }

    async fn ping(&self) -> PingResult {
        let head = b"j-wy";

        match self.socket.send_to(head, &self.remote_addr).await {
            Ok(4) => {}
            _ => return PingResult::SendFailed,
        };

        let mut buf: [u8; 32] = [0u8; 32];

        let sleep = tokio::time::sleep(Duration::from_secs(3));
        tokio::pin!(sleep);

        loop {
            tokio::select! {
                _ = &mut sleep => {
                    return PingResult::Timeout;
                },
                res = self.socket.recv_from(&mut buf) => {
                    match res {
                        Ok((len, peer_addr)) => {
                            if peer_addr == self.remote_addr {
                                if &buf[0..4] == head {
                                    return PingResult::Success;
                                } else {
                                    return PingResult::Invalid;
                                }
                            }
                        }
                        Ok(_) => return PingResult::Invalid,
                        Err(_) => return PingResult::Invalid,
                    }
                }
            }
        }
    }
}

fn parse_address() -> Option<SocketAddr> {
    let mut args = std::env::args();

    let program_name = args.nth(0).unwrap();
    let program_name = Path::new(&program_name).file_stem().unwrap();
    let program_name = program_name.to_str().unwrap();

    let print_help = || println!("Useage: \n\t{} <ip:port>", program_name);

    if args.len() != 1 {
        print_help();
        return None;
    }

    match SocketAddr::from_str(&args.nth(0).unwrap()) {
        Ok(target) => Some(target),
        Err(e) => {
            println!("address invalid {}", e);
            print_help();
            None
        }
    }
}

#[tokio::main]
async fn main() {
    let target = match parse_address() {
        Some(v) => v,
        None => return,
    };

    let local_ip = match target {
        SocketAddr::V4(_) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        SocketAddr::V6(_) => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    };

    let socket = match UdpSocket::bind(SocketAddr::new(local_ip, 0)).await {
        Ok(v) => v,
        Err(e) => {
            println!("Create udp socket failed! {}", e);
            return;
        }
    };

    // if let Err(_) = socket.connect(target).await {
    //     println!("Can't Send message to the network {}", target);
    //     return;
    // }

    let client = PingClient::new(socket, target).await;

    let err_min_duration = Duration::from_millis(1000);
    let ok_min_duration = Duration::from_millis(20);

    loop {
        let begin = Instant::now();

        let res = client.ping().await;

        let elapsed = Instant::now().duration_since(begin);
        println!("{} {} {}ms", target, res, elapsed.as_millis());

        let waiting = match res {
            PingResult::Success => ok_min_duration,
            _ => err_min_duration,
        };

        if elapsed < waiting {
            tokio::time::sleep(err_min_duration).await;
        }
    }
}
