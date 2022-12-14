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

// ça manque de bits

fn handle_connection(mut stream: TcpStream) {
    println!("request from {}\n", &stream.peer_addr().unwrap());

    let mut packet_length = PacketLength::Reading(Varint{length: 0, data: 0 });

    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .bytes()
        .map(|byte| byte.unwrap())
        .enumerate()
        .take_while(|byte| {
            match &mut packet_length {
                PacketLength::Known(varint) => (byte.0 - varint.length) <= varint.data as usize,
                PacketLength::Reading(varint) => {
                    let data = byte.1;
                    varint.data += ((data & 0b01111111) << (7 * varint.length) ) as u64;
                    if data >= 128 {
                        varint.length += 1;
                    } else {
                        // println!("varint.length = {} , varint.data = {}", varint.length+1, varint.data);
                        packet_length = PacketLength::Known(Varint { length: varint.length, data: varint.data });
                    }
                    true
                }
            }
        })
        .map(|byte| byte.1)
        .collect();

        match http_request.iter().last().unwrap() {
            0b00000001 => println!("status request"),
            0b00000010 => println!("login request"),
            _ => println!("unknown request")
        }

        // for byte in http_request.into_iter() {
        //     println!("{:#10b}", byte);
        // }
}