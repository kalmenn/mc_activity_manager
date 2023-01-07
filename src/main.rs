mod minecraft_server_runner;
mod mc_protocol;

use minecraft_server_runner::McServer;
use mc_protocol::{
    Codec,
    Packet,
    v761_packets::*,
    generic_packets::{self, GenericPacket},
    ProtocolVersion,
};

use std::net::SocketAddr;
use tokio::{
    net::{TcpListener},
    io,
    task,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let socket: SocketAddr = "0.0.0.0:6969".parse().expect("this should be a valid socket");
    loop {
        {
            let listener = TcpListener::bind(socket)
                .await
                .expect("Couldn't bind to TCP socket");

            println!("\n\x1b[38;2;0;200;0mSpoofer listening on port {}\x1b[0m\n", socket.port());

            let (sender, mut reciever) = tokio::sync::mpsc::channel::<()>(1);

            // We handle connections and loop until we recieve a Login request
            loop{if tokio::select!(
                Ok((stream, address)) = listener.accept() => {
                    let sender = sender.clone();
                    
                    task::spawn(async move {
                        let address = format!("\x1b[38;5;14m{address}\x1b[0m");
                        println!("Connection from {}", address);

                        match async {
                            let status = |message: &str| {
                                println!("{} → {}", &address, message);
                            };

                            let mut codec = Codec::new_server(stream)?;

                            loop {match codec.read_packet().await? {
                                Packet::Generic(packet) => match packet {
                                    GenericPacket::Serverbound(generic_packets::serverbound::ServerboundPacket::Handshake(packet)) => {
                                        status(&format!("Switching state to: {}", packet.next_state));
                                    },
                                }
                                Packet::V761(V761Packet::ServerboundPacket(packet)) => match packet {
                                    ServerboundPacket::Status(packet) => {match packet {
                                        serverbound::StatusPacket::StatusRequest{} => {
                                            status("Requested status");
                                            let json_response = serde_json::json!({
                                                "description": [
                                                    {
                                                        "text": "Hors Ligne ...\n",
                                                        "color": "gold"
                                                    },
                                                    {
                                                        "text": "Connectez vous pour démarrer le serveur",
                                                        "color": "dark_green"
                                                    }
                                                ],
                                                "players": {
                                                    "max": 0,
                                                    "online": 1,
                                                    "sample": [
                                                        {
                                                            "name": "J'ai pas hacké je jure",
                                                            "id": "4566e69f-c907-48ee-8d71-d7ba5aa00d20"
                                                        }
                                                    ]
                                                },
                                                "version": {
                                                    "name": "1.19.3",
                                                    "protocol": 761
                                                }
                                            }).to_string();
                                            codec.send_packet(clientbound::StatusPacket::StatusResponse{ json_response }).await?;
                                            status("Sent status");
                                        },
                                        serverbound::StatusPacket::PingRequest{ payload } => {
                                            status("Requested ping");
                                            codec.send_packet(clientbound::StatusPacket::PingResponse{ payload }).await?;
                                            status("Sent pong");
                                            break io::Result::Ok(false)
                                        },
                                    }},
                                    ServerboundPacket::Login(packet) => {match packet {
                                        serverbound::LoginPacket::LoginStart { name, player_uuid } => {
                                            status(&format!(
                                                "Recieved login request from \x1b[38;5;14m{name}\x1b[0m{}",
                                                if let Some(uuid) = player_uuid {
                                                    format!(" with uuid: \x1b[38;5;14m{uuid:x}\x1b[0m")
                                                } else {
                                                    "".to_owned()
                                                }
                                            ));
                                            break io::Result::Ok(true)
                                        },
                                    }},
                                    other => break Err(io::Error::new(
                                        io::ErrorKind::Other,
                                        format!("got an unsupported packet: {other:?}")
                                    ))
                                },
                                other => break Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!("unsupported protocol version: {:?}", Option::<ProtocolVersion>::from(other))
                                )),
                            }}
                        }.await {
                            Ok(should_we_start) => {
                                println!("Closed connection to {address}");
                                if should_we_start {
                                    sender.send(()).await.expect("channel shouldn't close");
                                }
                            },
                            Err(err) => {
                                println!("Killed connection to {address} on error: {err}");
                            }
                        }
                    });
                    false // Don't start the server
                },
                _ = reciever.recv() => {
                    // There should always be at least one sender alive.
                    // But just in case, we return anyway if we recieve None
                    true // Start the server
                }
            ){break}}
        }
        {
            let mut server = McServer::with_args(
                "/bin/bash", 
                &[
                    "start.sh"
                ]
            ).unwrap();

            let exit_status = server.wait_for_exit().await.unwrap();
            println!("Server exited on status: {}", exit_status);
        }
    }
}