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
    time::{Duration, Instant},
};
use tokio::{
    net::{TcpListener, TcpStream},
    io::{self, BufReader, BufWriter, AsyncBufReadExt, AsyncWriteExt},
    task,
    process::Command,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let socket: SocketAddr = "127.0.0.1:6969".parse().expect("this should be a valid socket");

    loop {
        {
            let listener = TcpListener::bind(socket)
                .await
                .expect("Couldn't bind to TCP socket");

            println!("\n\x1b[38;2;0;200;0mSpoofer listening on port {}\x1b[0m\n", socket.port());

            let (start_sender, mut start_reciever) = tokio::sync::mpsc::channel::<()>(1);

            let mut line_buffer = String::new();
            let mut stdin_reader = BufReader::new(io::stdin());

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
                _ = stdin_reader.read_line(&mut line_buffer) => {
                    if line_buffer == "stop\n".to_owned() {
                        std::process::exit(0);
                    };
                    line_buffer.clear();
                    false
                }
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

            let mut last_activity = Instant::now();
            let mut number_of_nulls: u32 = 0;

            loop{tokio::select!(
                exit_status = mc_server.wait() => {
                    drop(mc_stdin);
                    break println!("\x1b[38;5;14mMinecraft server exited on status: {exit_status:?}\x1b[0m");
                },
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    match get_playercount(socket).await {
                        Err(err) => match err {
                            PlayercountError::GotNull => {
                                number_of_nulls += 1;
                                if number_of_nulls > 3 {
                                    println!("\x1b[38;5;11mWarning: Status response from the server doesn't include player count.\x1b[0m")
                                }
                            },
                            PlayercountError::Inbound => println!("\x1b[38;5;11mWarning: Could not query player count from minecraft server.\nThis is not your fault, it is responding in an incorrect way\x1b[0m"),
                            PlayercountError::IO(err) => println!("\x1b[38;5;11mWarning: Could not query player count from minecraft server. Got err:\x1b[0m {err}"),
                        },
                        Ok(playercount) => {
                            if playercount == 0 && last_activity.elapsed() >= Duration::from_secs(300) {
                                println!("\x1b[38;5;14mStopping Minecraft Server due to inactivity\x1b[0m");
                                write_line(&mut mc_stdin, "stop\n").await.expect("should have been able to forward input to minecraft server stdin");
                                drop(mc_stdin);
                                break println!("\x1b[38;5;14mMinecraft server exited on status: {:?}\x1b[0m", mc_server.wait().await);
                            } else if playercount != 0 {
                                last_activity = Instant::now();
                            }
                        }
                    }
                },
                _ = stdin_reader.read_line(&mut line_buffer) => {
                    if line_buffer == "spoof\n".to_owned() {
                        write_line(&mut mc_stdin, "stop\n").await.expect("should have been able to forward input to minecraft server stdin");
                    } else if line_buffer == "stop\n".to_owned() {
                        write_line(&mut mc_stdin, "stop\n").await.expect("should have been able to forward input to minecraft server stdin");

                        println!("\x1b[38;5;14mMinecraft server exited on status: {:?}\x1b[0m", mc_server.wait().await);

                        std::process::exit(0);
                    } else {
                        write_line(&mut mc_stdin, &line_buffer).await.expect("should have been able to forward input to minecraft server stdin")
                    }
                    line_buffer.clear();
                },
            )}
        }
    }
}

async fn write_line(stdin: &mut tokio::process::ChildStdin, line: &str) -> io::Result<()> {
    stdin.write_all(line.as_bytes()).await?;
    stdin.flush().await
}

enum PlayercountError {
    GotNull,
    Inbound,
    IO(io::Error)
}

impl From<io::Error> for PlayercountError {
    fn from(err: io::Error) -> Self {
        PlayercountError::IO(err)
    }
}

async fn get_playercount(address: SocketAddr) -> Result<u64, PlayercountError> {
    let mut stream = TcpStream::connect(address).await?;
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
    ).await?
    .serialize_write(&mut writer).await?;

    writer.flush().await?;

    LengthPrefixed::from_mc_protocol(
        serverbound_packets::v760_packets::StatusPacket::StatusRequest{}
    ).await?
    .serialize_write(&mut writer).await?;

    writer.flush().await?;

    let packet = {
        let mut packet_reader = get_length_prefixed_reader(&mut reader).await
            .map_err(|_| PlayercountError::Inbound)?;
        clientbound_packets::v760_packets::StatusPacket::deserialize_read(&mut packet_reader).await
            .map_err(|_| PlayercountError::Inbound)?
    };

    if let clientbound_packets::v760_packets::StatusPacket::StatusResponse{ json_response } = packet {
        Ok(
            serde_json::from_str::<serde_json::Value>(&json_response)
            .map_err(|_| PlayercountError::Inbound)?
            ["players"]["online"].as_u64()
            .ok_or(PlayercountError::GotNull)?
        )
    } else {
        return Err(PlayercountError::Inbound)
    }
}