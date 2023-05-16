use std::collections::LinkedList;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::{UdpSocket, ToSocketAddrs};

struct UdpClient {
    socket: UdpSocket,
    responses: LinkedList<Response>,
}

impl UdpClient {
    async fn new() -> Self {
        Self {
            socket: UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
                .await
                .unwrap(),
            responses: LinkedList::new(),
        }
    }

    async fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], target: A) -> bool {
        self.socket.send_to(buf, target).await.is_ok()

    }
}

struct Response {
    peer_addr: SocketAddr,
    buf: Vec<u8>,
}
