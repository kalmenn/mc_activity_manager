mod mc_protocol;
use mc_protocol::{
    ServerCodec,
    ProtocolVersion,
    serverbound_packets::{self, Serverbound},
    clientbound_packets,
    data_types::{McVarint, LengthPrefixed, get_length_prefixed_reader},
    McProtocol,
};

use std::{
    net::SocketAddr, 
    process::Stdio,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    io::{self, BufReader, BufWriter, AsyncBufReadExt, AsyncWriteExt},
    task,
    process::Command,
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

                        let mut codec = ServerCodec::new(stream);

                        let output = async {loop {match codec.read_packet().await? {
                            Serverbound::Generic(packet) => match packet {
                                serverbound_packets::generic_packets::Generic::Handshake(packet) => {
                                    status(&format!("Switching state to: {}", packet.next_state));
                                },
                                serverbound_packets::generic_packets::Generic::ServerListPing(_) => {
                                    status("Recieved legacy server list ping");
                                    break Ok(false)
                                }
                            },
                            Serverbound::V760(packet) => match packet {
                                serverbound_packets::v760_packets::V760::Status(packet) => {match packet {
                                    serverbound_packets::v761_packets::StatusPacket::StatusRequest{} => {
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
                                                "name": "1.19.2",
                                                "protocol": 760
                                            }
                                        }).to_string();
                                        codec.send_packet(clientbound_packets::v760_packets::StatusPacket::StatusResponse{ json_response }).await?;
                                        status("Sent status");
                                    },
                                    serverbound_packets::v761_packets::StatusPacket::PingRequest{ payload } => {
                                        status("Requested ping");
                                        codec.send_packet(clientbound_packets::v760_packets::StatusPacket::PingResponse{ payload }).await?;
                                        status("Sent pong");
                                        break io::Result::Ok(false)
                                    },
                                }},
                                serverbound_packets::v760_packets::V760::Login(packet) => {match packet {
                                    serverbound_packets::v760_packets::LoginPacket::LoginStart { name, sig_data: _, player_uuid } => {
                                        status(&format!(
                                            "Recieved login request from \x1b[38;5;14m{name}\x1b[0m{}",
                                            if let Some(uuid) = player_uuid {
                                                format!(" with uuid: \x1b[38;5;14m{uuid:x}\x1b[0m")
                                            } else {
                                                "".to_owned()
                                            }
                                        ));
                                        codec.send_packet(clientbound_packets::v760_packets::LoginPacket::Disconnect { reason: serde_json::json!(
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
            ){
                // We exit the connection-handling loop whenever one of the branches returns true
                // and switch to the next state in the main loop (running the server)
                break 
            }}
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

            loop{tokio::select!(
                exit_status = mc_server.wait() => {
                    drop(mc_stdin);
                    break println!("Minecraft server exited on status: {:?}", exit_status);
                },
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    let response = {
                        let address = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 6969);
                        let mut stream = TcpStream::connect(address).await
                            .expect("should have been able to conenct to the minecraft server");
                        let (read_half, write_half) = stream.split();
                        let mut reader = BufReader::new(read_half);
                        let mut writer = BufWriter::new(write_half);

                        LengthPrefixed::from_mc_protocol(
                            serverbound_packets::generic_packets::HandshakePacket{
                                protocol_version: McVarint::from(760_i32),
                                server_address: "asd".to_owned(),
                                server_port: 25561,
                                next_state: serverbound_packets::generic_packets::NextState::Status,
                            }
                        ).await
                        .expect("this should be a valid packet")
                        .serialize_write(&mut writer).await
                        .expect("we should be able to write to the stream");

                        writer.flush().await.expect("stream should still be open");

                        LengthPrefixed::from_mc_protocol(serverbound_packets::v760_packets::StatusPacket::StatusRequest{}).await
                            .expect("this should be a valid packet")
                            .serialize_write(&mut writer).await
                            .expect("we should be able to write to the stream");

                        writer.flush().await.expect("stream should still be open");

                        let packet = {
                            let mut packet_reader = get_length_prefixed_reader(&mut reader).await
                                .expect("minecraft server should correctly encode packet length");
                            clientbound_packets::v760_packets::StatusPacket::deserialize_read(&mut packet_reader).await
                                .expect("minecraft server should correctly encode status response")
                        };

                        if let clientbound_packets::v760_packets::StatusPacket::StatusResponse{ json_response } = packet {
                            json_response
                        } else {
                            panic!("Warning! Server isn't responding in a valid way to status requests.")
                        }
                    };

                    let json_response: serde_json::Value = serde_json::from_str(&response).expect("minecraft should send valid json data");

                    println!("Online players: {}", json_response["players"]["online"])
                },
                _ = stdin_reader.read_line(&mut line_buffer) => {
                    mc_stdin.write_all(line_buffer.as_bytes()).await
                        .expect("should have been able to forward input to minecraft server stdin");
                    mc_stdin.flush().await
                        .expect("should have been able to flush minecraft server stdin");
                    line_buffer.clear();
                },
                // TODO: Stop server when player count has been 0 for over 5 minutes.
            )}
        }
    }
}