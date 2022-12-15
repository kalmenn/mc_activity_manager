use std::net::{TcpListener};

mod request;
mod varints;
mod clients;

use clients::*;
use request::*;

fn main() {
    let mut clients = ClientCache::new();

    let listener = TcpListener::bind("127.0.0.1:6969").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        clients.prune(2);

        let addr = stream.peer_addr().unwrap();
        let client = match clients.cache(addr) {
            Ok(client) => client,
            Err(err) => {
                eprintln!("{:?}", err);
                continue
            }
        };

        handle_connection(stream, client);
        println!("current cached clients: {:#?}", &clients);
    }
}