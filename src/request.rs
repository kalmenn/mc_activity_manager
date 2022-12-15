use std::{io::{prelude::*, BufReader}, net::TcpStream};

use crate::clients::Client;

struct Varint {
    length: usize,
    data: u64
}


#[derive(Debug)]
pub enum RequestState {
    Status,
    Login,
    Handshake
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
                        // println!("varint.length = {} , varint.data = {}", varint.length+1, varint.data);
                        packet.length.to_known();
                    }
                    true
                }
            }
        })
        .map(|byte| byte.1)
        .collect();

        println!("{:?}", &packet.body);

        stream.write_all("yo".as_bytes()).unwrap();
        stream.flush().unwrap();

        break
    }
        // match &mut packet.length {
        //     PacketLength::Reading(varint) => {
        //         varint.data += ((byte.1 & 0b01111111) << (7 * varint.length) ) as u64;
        //         varint.length += 1;
        //         if byte.1 < 128 {
        //             packet.length = PacketLength::Known(Varint{length: varint.length, data: varint.data});
        //         }
        //     },
        //     PacketLength::Known(varint) => {
        //         packet.body.push(byte.1);
        //         if packet.body.len() >= varint.data as usize {
        //             // println!("packet body:{:?}", packet.body);
        //             println!("recieved packet of length: {}", &varint.data);
        //             match request_state {
        //                 RequestState::Handshake => {
        //                     println!("reached state: Handshake");
        //                     match packet.body.iter().last().unwrap_or(&0) {
        //                         1 => {
        //                             request_state = RequestState::Status;
        //                             // TODO: set MODT in response
        //                         },
        //                         2 => {
        //                             request_state = RequestState::Login;
        //                             // TODO: disconnect player and display message
        //                         }
        //                         _ => return
        //                     }
        //                 },
        //                 RequestState::Login => {
        //                     println!("reached state: Login");
        //                     return
        //                 },
        //                 RequestState::Status => {
        //                     println!("reached state: Status");
        //                     return
        //                     // TODO: deal with ping requests
        //                 }
        //             }
        //             packet = Packet::empty();
        //         };
        //     }
        // };
    }