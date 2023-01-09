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
    net::{SocketAddrV4, Ipv4Addr}, 
    process::Stdio,
    time::{Duration, Instant},
    path::PathBuf,
};
use tokio::{
    net::{TcpListener, TcpStream},
    io::{self, BufReader, BufWriter, AsyncBufReadExt, AsyncWriteExt},
    task,
    process::Command,
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "Minecraft Server Activity Manager",
    author = "kalmenn <kalmenn@proton.me>",
    about = 
r#"Manages a minecraft server by automatically stopping it in periods of inactivity.

When no players have been online for more than the specified timeout, the minecraft server will be closed and activity manager will listen for incoming connections.
When someone tries to connect to the minecraft server, it will be started again.

Stdin is forwarded to the minecraft server, so you can still send commands. However, it is interpreted slightly:
- 'stop' will stop the minecraft server but also shut down the activity manager. This means it won't boot up automatically again.
  This is intended as a compatibility feature for any other managment script that might expect 'stop' to stop the whole process.
- 'spoof' will stop the minecraft server and enter the spoofing stage. It will start again when it recieves a connection."#,
)]
struct Cli {
    /// path to a script that starts your minecraft server
    start_script: PathBuf,

    /// the port your minecraft server listens on
    #[arg(long, short, default_value_t = 25565)]
    port: u16,

    /// the interface your minecraft server listens on
    #[arg(long, short, default_value_t = Ipv4Addr::new(0, 0, 0, 0))]
    interface: Ipv4Addr,

    /// the period of time (in minutes) activity manager will consider to be inactivity
    #[arg(long, short, default_value_t = 5)]
    timeout: u32
}

#[allow(clippy::single_match)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (stdin_sender, mut stdin_reciever) = tokio::sync::mpsc::channel::<String>(10);

    task::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut stdin_reader = BufReader::new(stdin);
        let mut line_buffer = String::new();
        loop {
            stdin_reader.read_line(&mut line_buffer).await.expect("should have been able to read from stdin");
            stdin_sender.send(line_buffer.clone()).await.expect("channel shouldn't close");
            if &line_buffer == "stop\n" {break};
            line_buffer.clear();
        }
    });

    let args = Cli::parse();

    let socket = SocketAddrV4::new(args.interface, args.port);

    loop {
        {
            let listener = match TcpListener::bind(socket).await {
                Ok(listener) => listener,
                Err(err) => {
                    println!("\x1b[38;5;11mCritical: Could not bind to socket {socket}. Got error: {err}\x1b[0m");
                    println!("\x1b[38;5;11mPlease ensure the interface and port are valid and not used by any other program\x1b[0m");
                    std::process::exit(1);
                }
            };

            println!("\n\x1b[38;2;0;200;0mSpoofer listening on port {}\x1b[0m\n", args.port);

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
                                serverbound_packets::generic_packets::Generic::ServerListPing(_) => {
                                    status("Recieved legacy server list ping");
                                    break Ok(false)
                                }
                                _ => {},
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
                                        status("Disconnected player");
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
                line = stdin_reciever.recv() => {
                    if &line.expect("channel shouldn't close") == "stop\n" {
                        std::process::exit(0);
                    };
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
                .args([args.start_script.as_os_str()])
                .stdin(Stdio::piped())
                .spawn()
                .expect("failed to start server in subprocess");

            let mut mc_stdin = mc_server.stdin.take().expect("should have been able to bind to minecraft server stdin");

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
                            PlayercountError::IO(err) => println!("\x1b[38;5;11mWarning: Could not reach minecraft server to query player count. Got err: {err}\x1b[0m"),
                        },
                        Ok(playercount) => {
                            if playercount == 0 && last_activity.elapsed() >= Duration::from_secs(u64::from(args.timeout) * 60) {
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
                line = stdin_reciever.recv() => {
                    let line = line.expect("channel shouldn't close");
                    if &line == "spoof\n" {
                        println!("\x1b[38;5;14mStopping minecraft server and entering spoofing mode\x1b[0m");

                        write_line(&mut mc_stdin, "stop\n").await.expect("should have been able to forward input to minecraft server stdin");
                    } else if &line == "stop\n" {
                        println!("\x1b[38;5;14mFully stopping the server\x1b[0m");

                        write_line(&mut mc_stdin, "stop\n").await.expect("should have been able to forward input to minecraft server stdin");

                        println!("\x1b[38;5;14mMinecraft server exited on status: {:?}\x1b[0m", mc_server.wait().await);

                        std::process::exit(0);
                    } else {
                        write_line(&mut mc_stdin, &line).await.expect("should have been able to forward input to minecraft server stdin")
                    }
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

async fn get_playercount(address: SocketAddrV4) -> Result<u64, PlayercountError> {
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
        Err(PlayercountError::Inbound)
    }
}