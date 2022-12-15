use std::{
    time::{Instant, Duration},
    net::SocketAddr,
    collections::HashMap
};

use crate::request::RequestState;

#[derive(Debug)]
pub struct Client {
    pub request_state: RequestState,
    pub last_seen: Instant,
    pub addr: SocketAddr
}

impl Client {
    pub fn new(addr: SocketAddr) -> Client {
        return Client {
            request_state: RequestState::Handshake,
            last_seen: Instant::now(),
            addr: addr
        };
    }
}

#[derive(Debug)]
pub struct ClientCache {
    clients: HashMap<SocketAddr, Client>
}

#[derive(Debug)]
pub enum ClientCacheError {
    ClientInsertError,
    ClientReadError
}

impl ClientCache {
    pub fn new() -> ClientCache {
        return ClientCache{clients: HashMap::<SocketAddr, Client>::new()};
    }

    /// Forgets clients that haven't been active in the last `timeout` seconds.
    pub fn prune(&mut self, timeout: u64) {
        self.clients.retain(|_, Client{last_seen, ..}| last_seen.elapsed() < Duration::from_secs(timeout));
    }

    pub fn cache<'a>(&'a mut self, addr: SocketAddr) -> Result<&'a mut Client, ClientCacheError> {
        return match self.clients.contains_key(&addr) {
            false => {
                self.clients.insert(addr, Client::new(addr));
                self.clients.get_mut(&addr).ok_or(ClientCacheError::ClientInsertError)
            },
            true => {
                match self.clients.get_mut(&addr) {
                    None => Err(ClientCacheError::ClientReadError),
                    Some(client) => {
                        client.last_seen = Instant::now();
                        Ok(client)
                    }
                }
            }
        };
    }
}