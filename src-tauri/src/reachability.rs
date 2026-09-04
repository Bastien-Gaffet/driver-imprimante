use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub fn is_reachable(ip: &str, port: u16, timeout: Duration) -> bool {
    let addr = match format!("{ip}:{port}").to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => return false,
        },
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, timeout).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::time::Duration;

    #[test]
    fn reachable_when_port_open() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        assert!(is_reachable("127.0.0.1", port, Duration::from_millis(500)));
    }

    #[test]
    fn unreachable_when_port_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        assert!(!is_reachable("127.0.0.1", port, Duration::from_millis(200)));
    }
}
