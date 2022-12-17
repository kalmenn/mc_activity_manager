mod varint;
mod codec;
mod threadpool;

use std::{io, net::{TcpStream, TcpListener}};
use std::sync::mpsc;

use varint::into_varint;
use codec::Codec;
use threadpool::ThreadPool;

enum Request {
    Status,
    Start
}

pub fn wait_for_start_request() {
    let listener = TcpListener::bind("127.0.0.1:6969").unwrap();
    listener.set_nonblocking(true).unwrap();
    println!("\n\x1b[38;2;0;200;0mSpoofer listening on port 6969\x1b[0m\n");

    let pool = ThreadPool::new(10);

    let (tx_request, rx_request) = mpsc::channel::<Request>();

    loop {
        if let Ok((stream, address)) = listener.accept() {
            let tx_request = tx_request.clone();
            let address = format!("\x1b[38;5;14m{}\x1b[0m", address);
            pool.execute(move || {
                println!("Connection from {}", address);
                match handle_connection(stream) {
                    Ok(request) => {
                        println!("Closed connection to {address}");
                        tx_request.send(request).unwrap();
                    },
                    Err(err) => println!("Killed connection to {address} on error: {err}")
                }
            });
        }
        if let Ok(Request::Start) = rx_request.try_recv() {
            break
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

enum RequestState {
    Handshake,
    Login,
    Status
}

fn handle_connection(stream: TcpStream) -> io::Result<Request>{
    let address = format!("\x1b[38;5;14m{}\x1b[0m", &stream.peer_addr()?);

    let status = |status: &str| {
        println!("{} → {}", address, status);
    };

    let mut request_state = RequestState::Handshake;

    let mut codec = Codec::new(stream)?;
    loop {
        let message = codec.read_message()?;

        match request_state {
            RequestState::Handshake => {
                status("State: Handshaking");
                match &message.iter().last() {
                    Some(1) => {
                        request_state = RequestState::Status;
                        status("Switching state to: Status");
                    },
                    Some(2) => {
                        request_state = RequestState::Login;
                        status("Switching state to: Login");
                    }
                    _ => {
                        status("Garbled packet");
                        return Err(io::Error::from(io::ErrorKind::InvalidData));
                    }
                }
            },
            RequestState::Status => {
                status("State: Status");
                match &message[0] {
                    0 => {
                        status("Requested status");
                        let mut text = "{\"description\":[{\"text\":\"Hors Ligne ...\n\",\"color\":\"gold\"},{\"text\":\"Connectez vous pour démarrer le serveur\",\"color\":\"dark_green\"}],\"players\":{\"max\":0,\"online\":1,\"sample\":[{\"name\":\"J'ai pas hacké je jure\",\"id\":\"4566e69f-c907-48ee-8d71-d7ba5aa00d20\"}]},\"version\":{\"name\":\"1.19.2\",\"protocol\":760}}"
                        .as_bytes().to_vec();
                        
                        let mut response = Vec::<u8>::new();
                        response.push(0);
                        response.append(&mut into_varint(text.len()));
                        response.append(&mut text);
                        
                        codec.send_message(response)?;
                        status("sent status");
                    }
                    1 => {
                        status("Requested ping");
                        codec.send_message(message)?;
                        status("Sent pong");
                        return Ok(Request::Status);
                    }
                    _ => {
                        status("Garbled packet");
                        return Err(io::Error::from(io::ErrorKind::InvalidData));
                    }
                }
            }
            RequestState::Login => {
                status("Requested login");
                let mut text = "[{\"text\":\"Serveur Hors Ligne\n\n\",\"color\":\"red\"},{\"text\":\"Demande de démarrage reçue,\nle serveur devrait être disponible d'ici une minute\",\"color\":\"white\"}]"
                .as_bytes().to_vec();
                
                let mut response = Vec::<u8>::new();
                response.push(0);
                response.append(&mut into_varint(text.len()));
                response.append(&mut text);
                
                codec.send_message(response)?;
                status("Sent disconnect message");

                return Ok(Request::Start);
            }
        }
    }
}