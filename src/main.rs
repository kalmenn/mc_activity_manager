mod mc_protocol;
use mc_protocol::{
    Codec,
    Packet,
    v761_packets::*,
    generic_packets::{self, GenericPacket},
    ProtocolVersion,
};

use std::{
    net::SocketAddr, 
    process::Stdio,
};
use tokio::{
    net::TcpListener,
    io::{self, BufReader, AsyncBufReadExt, AsyncWriteExt},
    task,
    process::Command,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (sigint_sender, sigint_reciever) = tokio::sync::mpsc::channel::<()>(1);
    let mut sigint_reciever_holder = Some(sigint_reciever);

    task::spawn(async move {
        tokio::signal::ctrl_c().await.expect("should be able to bind to incoming SIGINT stream");
        println!("\nCaught SIGINT event");
        sigint_sender.send(()).await.expect("internal sigint channel shouldn't close");
    });

    let socket: SocketAddr = "0.0.0.0:6969".parse().expect("this should be a valid socket");
    loop {
        {
            let mut sigint_reciever = sigint_reciever_holder.take()
                .expect("sigint reciever should have been put back from the previous take");

            let listener = TcpListener::bind(socket)
                .await
                .expect("Couldn't bind to TCP socket");

            println!("\n\x1b[38;2;0;200;0mSpoofer listening on port {}\x1b[0m\n", socket.port());

            let (start_sender, mut start_reciever) = tokio::sync::mpsc::channel::<()>(1);

            // We handle connections and loop until we recieve a Login request
            loop{if tokio::select!(
                Ok((stream, address)) = listener.accept() => {
                    let start_sender = start_sender.clone();

                    task::spawn(async move {
                        let address = format!("\x1b[38;5;14m{address}\x1b[0m");
                        println!("Connection from {}", address);

                        let status = |message: &str| {
                            println!("{} → {}", &address, message);
                        };

                        let mut codec = Codec::new_server(stream);

                        let output = async {loop {match codec.read_packet().await? {
                            Packet::Generic(GenericPacket::Serverbound(packet)) => match packet {
                                generic_packets::serverbound::ServerboundPacket::Handshake(packet) => {
                                    status(&format!("Switching state to: {}", packet.next_state));
                                },
                                generic_packets::serverbound::ServerboundPacket::ServerListPing(_) => {
                                    status("Recieved legacy server list ping");
                                    break Ok(false)
                                }
                            }
                            Packet::V761(V761Packet::ServerboundPacket(packet)) => match packet {
                                ServerboundPacket::Status(packet) => {match packet {
                                    serverbound::StatusPacket::StatusRequest{} => {
                                        status("Requested status");
                                        let json_response = serde_json::json!({
                                            "description": [
                                                {
                                                    "text": "Hors Ligne\n",
                                                    "color": "dark_red"
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
                                        codec.send_packet(clientbound::LoginPacket::Disconnect { reason: serde_json::json!(
                                            [
                                                {
                                                    "text": "Serveur Hors Ligne\n\n",
                                                    "color": "red"
                                                },
                                                {
                                                    "text": "Demande de démarrage reçue,\nle serveur devrait être disponible d'ici une minute",
                                                    "color": "white"
                                                }
                                            ]
                                        ).to_string()}).await?;
                                        status("Sent disconnect message");
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
                                format!("unsupported protocol version: {}", Option::<ProtocolVersion>::from(other).expect("we already matched against generic packets"))
                            )),
                        }}}.await;

                        match output {
                            Ok(should_we_start) => {
                                println!("Closed connection to {address}");
                                if should_we_start {
                                    start_sender.send(()).await.expect("channel shouldn't close");
                                }
                            },
                            Err(err) => {
                                println!("Killed connection to {address} on error: {err}");
                            }
                        };
                    });

                    false // Don't start the server
                },
                _ = start_reciever.recv() => {
                    // There should always be at least one sender alive.
                    // But just in case, we return anyway if we recieve None

                    true // Start the server
                },
                _ = sigint_reciever.recv() => {
                    return println!("Exiting")
                }
            ){
                // We exit the connection-handling loop whenever one of the branches returns true
                // and switch to the next state in the main loop (running the server)
                break 
            }}

            sigint_reciever_holder = Some(sigint_reciever);
        }
        {
            println!("\n\x1b[38;2;0;200;0mStarting minecraft server as child process\x1b[0m\n");

            let mut mc_server = Command::new("/bin/bash")
                .args(["./start.sh"])
                .stdin(Stdio::piped())
                .spawn()
                .expect("failed to start server in subprocess");

            let mut mc_stdin = mc_server.stdin.take().expect("should have been able to bind to minecraft server stdin");
            let mut stdin_reader = BufReader::new(io::stdin());
            let mut line_buffer = String::new();

            let mut sigint_reciever = sigint_reciever_holder.take()
                .expect("sigint reciever should have been put back from the previous take");

            loop{tokio::select!(
                exit_status = mc_server.wait() => {
                    break println!("Server exited on status: {:?}", exit_status);
                },
                _ = sigint_reciever.recv() => {
                    // TODO: This doesn't seem to actually gracefully stop the minecraft server
                    println!("Stopping minecraft server");
                    mc_stdin.write_all("stop\n".as_bytes()).await
                        .expect("should have been able to write to minecraft server stdin");
                    mc_stdin.flush().await
                        .expect("should have been able to flush minecraft server stdin");
                    println!(
                        "Minecraft Server exited with status: {}",
                        mc_server.wait().await.expect("minecraft server should have been running")
                    );
                    return
                },
                _ = stdin_reader.read_line(&mut line_buffer) => {
                    mc_stdin.write_all(line_buffer.as_bytes()).await
                        .expect("should have been able to forward input to minecraft server stdin");
                    mc_stdin.flush().await
                        .expect("should have been able to flush minecraft server stdin");
                    line_buffer.clear();
                },
                // TODO: query player count and stop server when nobody has been online for over 5 minutes.
            )}

            sigint_reciever_holder = Some(sigint_reciever);
        }
    }
}