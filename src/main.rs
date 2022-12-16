use std::net::TcpListener;

mod handler;
mod varint;
mod codec;

use handler::*;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6969").unwrap();

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                println!("Continued on error: {:?}", e);
                continue;
            }
        };

        match handle_connection(stream) {
            Ok(()) => println!("Connection closed"),
            Err(err) => println!("killed connection on error: {}", err)
        }
    }
}