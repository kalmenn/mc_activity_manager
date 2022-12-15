use std::{
    time::{Instant, Duration},
    net::SocketAddr,
    collections::HashMap
};

use crate::request::RequestState;

#[derive(Debug)]
pub struct Client {
    pub request_state: RequestState,
    pub last_seen: Instant
}

impl Client {
    pub fn new() -> Client {
        return Client {
            request_state: RequestState::Handshake,
            last_seen: Instant::now()
        };
    }
}

#[derive(Debug)]
pub struct ClientCache {
    clients: HashMap<SocketAddr, Client>
}

impl ClientCache {
    pub fn new() -> ClientCache {
        return ClientCache{clients: HashMap::<SocketAddr, Client>::new()};
    }

    // pub fn prune(&mut self) {
    //     self.clients.retain(|_, Client{ request_state: _, last_seen }| last_seen.elapsed() < Duration::from_secs(5));
    // }

    pub fn cache<'a>(&'a mut self, addr: SocketAddr) -> &'a mut Client {
        return match self.clients.contains_key(&addr) {
            false => {
                self.clients.insert(addr, Client::new());
                self.clients.get_mut(&addr).unwrap()
            },
            true => {
                let client = self.clients.get_mut(&addr).unwrap();
                client.last_seen = Instant::now();
                client
            }
        };
    }
}