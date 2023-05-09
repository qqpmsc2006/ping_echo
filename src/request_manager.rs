use std::collections::HashMap;
use std::sync::Mutex;

pub struct RequestManager {
    requests: Mutex<HashMap<std::net::SocketAddr, tokio::sync::oneshot::Sender<bool>>>,
}

impl RequestManager {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, addr: std::net::SocketAddr, tx: tokio::sync::oneshot::Sender<bool>) {
        let mut r = self.requests.lock().unwrap();
        r.insert(addr, tx);
    }

    pub fn unregister(&self, addr: &std::net::SocketAddr) {
        let mut r = self.requests.lock().unwrap();
        r.remove(addr);
    }

    pub fn notify(&self, addr: std::net::SocketAddr, res: bool) {
        let mut r = self.requests.lock().unwrap();
        if let Some(tx) = r.remove(&addr) {
            let _ = tx.send(res);
        }
    }

    pub fn is_empty(&self) -> bool {
        let r = self.requests.lock().unwrap();
        r.is_empty()
    }
}
