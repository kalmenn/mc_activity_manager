use std::{io::{prelude::*, BufReader}, net::TcpStream};

use crate::clients::Client;
use crate::varints::into_varint;

#[derive(Debug)]
pub enum RequestState {
    Handshake,
    Login,
    Status,
    _Ping
}

enum PacketLength {
    Known(PacketLengthVarint),
    Reading(PacketLengthVarint)
}

struct PacketLengthVarint {
    length: usize,
    data: u64
}

impl PacketLength {
    fn to_known(&mut self) {
        if let Self::Reading(varint) = self {
            *self = Self::Known(PacketLengthVarint{length: varint.length, data: varint.data});
        }
    }
}

struct Packet {
    length: PacketLength,
    body: Vec<u8>
}

impl Packet {
    /// Returns an empty packet
    fn empty() -> Packet {
        return Packet{
            length: PacketLength::Reading(PacketLengthVarint{length: 0, data: 0}),
            body: Vec::<u8>::new()
        };
    }
}

pub fn handle_connection(mut stream: TcpStream, client: &mut Client) {
    println!("request from {}", &stream.peer_addr().unwrap());

    let mut packet: Packet;

    loop {
        packet = Packet::empty();

        packet.body = BufReader::new(&mut stream)
        .bytes()
        .map(|byte| byte.unwrap())
        .enumerate()
        .take_while(|byte| {
            match &mut packet.length {
                PacketLength::Known(varint) => (byte.0 - varint.length) <= varint.data as usize,
                PacketLength::Reading(varint) => {
                    let data = byte.1;
                    varint.data += ((data & 0b01111111) << (7 * varint.length) ) as u64;
                    if data >= 128 {
                        varint.length += 1;
                    } else {
                        packet.length.to_known();
                    }
                    true
                }
            }
        })
        .map(|byte| byte.1)
        .collect();

        match &client.request_state {
            RequestState::Handshake => {
                println!("{:?}: Expecting handshake ...", &stream.peer_addr());
                match &packet.body.bytes().last() {
                    Some(Ok(1)) => {
                        println!("{:?}: Requested Status", &stream.peer_addr());
                        client.request_state = RequestState::Status;
                    }
                    Some(Ok(2)) => {
                        println!("{:?}: Requested Login", &stream.peer_addr());
                        client.request_state = RequestState::Login;
                    }
                    _ => {
                        println!("{:?}: Garbled packet", &stream.peer_addr());
                        break
                    }
                }
            },
            RequestState::Status => {
                println!("{:?}: Expecting status request ...", &stream.peer_addr());

                println!("Packet body: {:?}", &packet.body);

                match packet.body.as_slice() {
                    &[1, 0] => {
                        let mut message = "{\"previewsChat\":false,\"enforcesSecureChat\":true,\"description\":{\"text\":\"A Minecraft Server\"},\"players\":{\"max\":20,\"online\":0},\"version\":{\"name\":\"1.19.2\",\"protocol\":760}}"
                        .as_bytes().to_vec();
                        
                        let mut message_length = into_varint(message.len());
                        
                        let mut response = into_varint(message.len() + message_length.len());
                        
                        response.push(0);
                        response.append(&mut message_length);
                        response.append(&mut message);
                        
                        match stream.write_all(&response) {
                            Ok(_) => println!("{:?}: Wrote to stream", &stream.peer_addr()),
                            Err(err) => {
                                eprintln!("Got error while writing to stream: {}", err);
                                break
                            }
                        }
                        match stream.flush() {
                            Ok(_) => println!("{:?}: Response sent", &stream.peer_addr()),
                            Err(err) => {
                                eprintln!("Got error while sending response: {}", err);
                                break
                            }
                        }
                    }
                    _ => {
                        println!("{:?}: Garbled packet", &stream.peer_addr());
                        break
                    }
                }
            }
            // TODO: implement next states
            _ => break
        }
    }

    match stream.peer_addr() {
        Ok(addr) => println!("Killed connection to {}", addr),
        Err(err) => eprintln!("killed connection {}", err)
    }
}