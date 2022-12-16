use std::{io, net::TcpStream};

use crate::varints::into_varint;
use crate::codec::Codec;

#[derive(Debug)]
pub enum RequestState {
    Handshake,
    Login,
    Status,
    Ping
}

pub fn handle_connection(stream: TcpStream) -> io::Result<()>{
    println!("request from {}", &stream.peer_addr().unwrap());
    
    let mut codec = Codec::new(stream)?;
    loop {
        println!("got packet: {:?}", codec.read_message()?.iter().map(|byte| format!("{:#x}", byte)).collect::<Vec<String>>());

        let mut text = "{\"previewsChat\":false,\"enforcesSecureChat\":true,\"description\":{\"text\":\"Currently offline ...\",\"color\":\"red\"},\"players\":{\"max\":0,\"online\":0},\"version\":{\"name\":\"1.19.2\",\"protocol\":760}}"
        .as_bytes().to_vec();

        let mut message = Vec::<u8>::new();
        message.push(0);
        message.append(&mut into_varint(text.len()));
        message.append(&mut text);

        codec.send_message(message)?;

        // match &client.request_state {
        //     RequestState::Handshake => {
        //         println!("{:?}: Expecting handshake ...", &stream.peer_addr());
        //         match &packet.body.bytes().last() {
        //             Some(Ok(1)) => {
        //                 println!("{:?}: Requested Status", &stream.peer_addr());
        //                 client.request_state = RequestState::Status;
        //             }
        //             Some(Ok(2)) => {
        //                 println!("{:?}: Requested Login", &stream.peer_addr());
        //                 client.request_state = RequestState::Login;
        //             }
        //             _ => {
        //                 println!("{:?}: Garbled packet", &stream.peer_addr());
        //                 break
        //             }
        //         }
        //     },
        //     RequestState::Status => {
        //         println!("{:?}: Expecting status request ...", &stream.peer_addr());

        //         println!("Packet body: {:?}", &packet.body);

        //         match packet.body.as_slice() {
        //             &[1, 0] => {
        //                 let mut message = "{\"previewsChat\":false,\"enforcesSecureChat\":true,\"description\":{\"text\":\"A Minecraft Server\"},\"players\":{\"max\":20,\"online\":0},\"version\":{\"name\":\"1.19.2\",\"protocol\":760}}"
        //                 .as_bytes().to_vec();
                        
        //                 let mut message_length = into_varint(message.len());
                        
        //                 let mut response = into_varint(message.len() + message_length.len());
                        
        //                 response.push(0);
        //                 response.append(&mut message_length);
        //                 response.append(&mut message);
                        
        //                 match stream.write_all(&response) {
        //                     Ok(_) => println!("{:?}: Wrote to stream", &stream.peer_addr()),
        //                     Err(err) => {
        //                         eprintln!("Got error while writing to stream: {}", err);
        //                         break
        //                     }
        //                 }
        //                 match stream.flush() {
        //                     Ok(_) => println!("{:?}: Response sent", &stream.peer_addr()),
        //                     Err(err) => {
        //                         eprintln!("Got error while sending response: {}", err);
        //                         break
        //                     }
        //                 }
        //             }
        //             _ => {
        //                 println!("{:?}: Garbled packet", &stream.peer_addr());
        //                 break
        //             }
        //         }
        //     }
        //     // TODO: implement next states
        //     _ => break
        // }
    }

    // match stream.peer_addr() {
    //     Ok(addr) => println!("Killed connection to {}", addr),
    //     Err(err) => eprintln!("killed connection {}", err)
    // }
}