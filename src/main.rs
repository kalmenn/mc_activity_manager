use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6969").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

use std::io::{prelude::*, BufReader};

struct Varint {
    length: usize,
    data: u64
}

enum PacketLength {
    Known(Varint),
    Reading(Varint)
}

impl PacketLength {
    fn to_known(&mut self) {
        if let Self::Reading(varint) = self {
            *self = Self::Known(Varint{length: varint.length, data: varint.data});
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
            length: PacketLength::Reading(Varint{length: 0, data: 0}),
            body: Vec::<u8>::new()
        };
    }
}

#[derive(Debug)]
enum RequestState {
    Status,
    Login,
    Handshake
}

fn handle_connection(mut stream: TcpStream) {
    println!("request from {}\n", &stream.peer_addr().unwrap());

    let mut packet = Packet::empty();
    
    let mut request_state = RequestState::Handshake;

    for byte in BufReader::new(&mut stream)
        .bytes()
        .map(|byte| byte.unwrap())
        .enumerate() 
    {
        match &mut packet.length {
            PacketLength::Reading(varint) => {
                varint.data += ((byte.1 & 0b01111111) << (7 * varint.length) ) as u64;
                varint.length += 1;
                if byte.1 < 128 {
                    packet.length = PacketLength::Known(Varint{length: varint.length, data: varint.data});
                }
            },
            PacketLength::Known(varint) => {
                packet.body.push(byte.1);
                if packet.body.len() >= varint.data as usize {
                    // println!("packet body:{:?}", packet.body);
                    println!("recieved packet of length: {}", &varint.data);
                    match request_state {
                        RequestState::Handshake => {
                            println!("reached state: Handshake");
                            match packet.body.iter().last().unwrap_or(&0) {
                                1 => {
                                    request_state = RequestState::Status;
                                    // TODO: set MODT in response
                                },
                                2 => {
                                    request_state = RequestState::Login;
                                    // TODO: disconnect player and display message
                                }
                                _ => return
                            }
                        },
                        RequestState::Login => {
                            println!("reached state: Login");
                            return
                        },
                        RequestState::Status => {
                            println!("reached state: Status");
                            return
                            // TODO: deal with ping requests
                        }
                    }
                    packet = Packet::empty();
                };
            }
        };
    }
}