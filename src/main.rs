use std::net::TcpListener;

mod handler;
mod varint;
mod codec;

use handler::*;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6969").unwrap();

    println!("\x1b[38;2;0;200;0mListening on port 6969\x1b[0m\n");

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(e) => {
                println!("Continued on error: {:?}", e);
                continue;
            }
        };

        match handle_connection(stream) {
            Ok(request) => {
                println!("Connection closed\n");
                if let Request::Start = request {
                    println!("\x1b[38;2;0;200;0mExiting and letting the server start\x1b[0m");
                    break
                }
            },
            Err(err) => println!("Killed connection on error: {}", err)
        }
    }
}