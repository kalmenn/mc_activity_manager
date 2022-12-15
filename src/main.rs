use std::net::{TcpListener};

mod request;
mod varints;
mod clients;
mod codec;

use clients::*;
use request::*;

// https://stackoverflow.com/questions/49785136/is-there-a-shortcut-to-unwrap-or-continue-in-a-loop
macro_rules! continue_on_err {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                println!("Continued on error: {:?}", e);
                continue;
            }
        }
    };
}

fn main() {
    let mut clients = ClientCache::new();

    let listener = TcpListener::bind("127.0.0.1:6969").unwrap();

    for stream in listener.incoming() {
        let stream = continue_on_err!(stream);

        clients.prune(2);

        let addr = continue_on_err!(stream.peer_addr());
        let client = continue_on_err!(clients.cache(addr));

        handle_connection(stream, client);
        println!("current cached clients: {:#?}", &clients);
    }
}