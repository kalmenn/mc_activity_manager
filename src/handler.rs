use std::{io, net::TcpStream};

use crate::varint::into_varint;
use crate::codec::Codec;

enum RequestState {
    Handshake,
    Login,
    Status
}

pub fn handle_connection(stream: TcpStream) -> io::Result<()>{
    let address = &stream.peer_addr()?;
    println!("request from {}", address);

    let mut request_state = RequestState::Handshake;

    let mut codec = Codec::new(stream)?;
    loop {
        let message = codec.read_message()?;

        println!("{:?} → got packet: {:?}", address, &message.iter().map(|byte| format!("{:#x}", byte)).collect::<Vec<String>>());

        match request_state {
            RequestState::Handshake => {
                println!("{:?} → State: Handshaking", address);
                match &message.iter().last() {
                    Some(1) => {
                        request_state = RequestState::Status;
                        println!("{:?} → Switching state to: Status", address);
                    },
                    Some(2) => {
                        request_state = RequestState::Login;
                        println!("{:?} → Switching state to: Login", address);
                    }
                    _ => {
                        println!("{:?} → Garbled packet", address);
                        return Err(io::Error::from(io::ErrorKind::InvalidData));
                    }
                }
            },
            RequestState::Status => {
                println!("{:?} → State: Status", address);
                match &message[0] {
                    0 => {
                        println!("{:?} → Requested status", address);
                        let mut text = "{\"description\":[{\"text\":\"Actuellement Hors Ligne ...\n\",\"color\":\"gold\"},{\"text\":\"Connectez vous pour démarrer le serveur\",\"color\":\"dark_green\"}],\"players\":{\"max\":0,\"online\":1,\"sample\":[{\"name\":\"J'ai pas hacké je jure\",\"id\":\"4566e69f-c907-48ee-8d71-d7ba5aa00d20\"}]},\"version\":{\"name\":\"1.19.2\",\"protocol\":760}}"
                        .as_bytes().to_vec();
                        
                        let mut response = Vec::<u8>::new();
                        response.push(0);
                        response.append(&mut into_varint(text.len()));
                        response.append(&mut text);
                        
                        codec.send_message(response)?;
                        println!("{:?} → sent placeholder message", address);
                    }
                    1 => {
                        println!("{:?} → Requested ping", address);
                        codec.send_message(message)?;
                        println!("{:?} → Sent pong", address);
                        return Ok(());
                    }
                    _ => {
                        println!("{:?} → Garbled packet", address);
                        return Err(io::Error::from(io::ErrorKind::InvalidData));
                    }
                }
            }
            RequestState::Login => {
                return Ok(())
            }
        }
    }
}