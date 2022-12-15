use std::net::{TcpListener, SocketAddr};

mod request;
mod clients;

use clients::*;
use request::*;

fn main() {
    let mut clients = ClientCache::new();

    let listener = TcpListener::bind("127.0.0.1:6969").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        // clients.prune();

        let addr = stream.peer_addr().unwrap();
        let client = clients.cache(addr);

        handle_connection(stream, client);
        println!("current cached clients: {:?}", &clients);
    }
}