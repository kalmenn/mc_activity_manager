use std::{io::{prelude::*, BufReader}, net::TcpStream};

use crate::clients::Client;

struct Varint {
    length: usize,
    data: u64
}

#[derive(Debug)]
pub enum RequestState {
    Handshake,
    Login,
    Status,
    Ping
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

pub fn handle_connection(mut stream: TcpStream, client: &mut Client) {
    println!("request from {}", &stream.peer_addr().unwrap());

    loop {
        let mut packet: Packet = Packet::empty();

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
                println!("{:?}: Handshaking", &stream.peer_addr());
                match packet.body.bytes().last() {
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
            // TODO: implement next states
            _ => {
                break
            }
        }

        // match stream.write_all("yo".as_bytes()) {
        //     Ok(_) => println!("Sent response"),
        //     Err(err) => {
        //         eprintln!("{}", err);
        //         break
        //     }

        // }
        // stream.flush().unwrap();

        packet = Packet::empty();
    }

    match stream.peer_addr() {
        Ok(addr) => println!("Killed connection to {}", addr),
        Err(err) => eprintln!("killed connection {}", err)
    }
    }