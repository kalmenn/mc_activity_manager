use std::net::TcpListener;

mod handler;
mod varint;
mod codec;
mod minecraft_server;

use handler::*;

fn main() {
    loop {
        let listener = TcpListener::bind("127.0.0.1:6969").unwrap();
        println!("\x1b[38;2;0;200;0mListening on port 6969\x1b[0m\n");
        
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(e) => {
                    println!("Skipped connection on error: {:?}", e);
                    continue;
                }
            };
            match handle_connection(stream) {
                Ok(request) => {
                    println!("╰ Connection closed\n");
                    if let Request::Start = request {break};
                },
                Err(err) => println!("╰ Killed connection on error: {}", err)
            }
        }
        
        drop(listener);
        
        println!("\x1b[38;2;0;200;0mStarting minecraft server in child process\x1b[0m\n");
        println!("Exit status: {:?}\n", minecraft_server::run_server());
    }
}