mod mc_protocol;
use mc_protocol::{
    clientbound_packets::v760_packets as clientbound,
    data_types::{get_length_prefixed_reader, LengthPrefixed, McVarint},
    serverbound_packets::{generic_packets, v760_packets as serverbound, Serverbound},
    McProtocol, ProtocolVersion, ServerCodec,
};

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    fs,
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    process::Command,
    task,
};

use clap::Parser;

use chrono::Local;

#[derive(Parser, Debug)]
#[command(
    name = "Minecraft Server Activity Manager",
    author = "kalmenn <kalmenn@proton.me>",
    about = r#"Manages a minecraft server by automatically stopping it in periods of inactivity.

When no players have been online for more than the specified timeout, the minecraft server will be closed and activity manager will listen for incoming connections.
When someone tries to connect to the minecraft server, it will be started again.

Stdin is forwarded to the minecraft server, so you can still send commands. However, it is interpreted slightly:
- 'stop' will stop the minecraft server but also shut down the activity manager. This means it won't boot up automatically again.
   This is intended as a compatibility feature for any other managment script that might expect 'stop' to stop the whole process.
- 'spoof' will stop the minecraft server and enter the spoofing stage. It will start again when it recieves a connection.
- 'start' only works in the spoofing stage and starts the minecraft server whether someone tried to connect or not"#
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
    timeout: u32,

    /// Root folder of your minecraft server.
    #[arg(long, short = 'r')]
    server_root: Option<PathBuf>,

    /// if set, activity manager will only start the minecraft server for players present in the provided whitelist.json or ops.json.
    #[arg(long, short, requires = "server_root")]
    whitelist: bool,
}

const LOGIN_RESPONSE: &str = r#"[{"text":"Serveur Hors Ligne\n\n","color":"red"},{"text":"Demande de démarrage reçue,\nle serveur devrait être disponible d'ici une minute","color":"white"}]"#;
const STATUS_RESPONSE: &str = r#"{"description":[{"text":"Hors Ligne\n","color":"dark_red"},{"text":"Connectez vous pour démarrer le serveur","color":"dark_green"}],"version":{"name":"1.19.2","protocol":760}}"#;

const TIME_FORMAT: &str = "[%H:%M:%S]";

#[allow(clippy::single_match)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (stdin_sender, mut stdin_reciever) = tokio::sync::mpsc::channel::<String>(10);

    task::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut stdin_reader = BufReader::new(stdin);
        let mut line_buffer = String::new();
        loop {
            stdin_reader
                .read_line(&mut line_buffer)
                .await
                .expect("should have been able to read from stdin");
            stdin_sender
                .send(line_buffer.clone())
                .await
                .expect("channel shouldn't close");
            if &line_buffer == "stop\n" {
                break;
            };
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

            let whitelist = if args.whitelist {
                match parse_whitelist(
                    args.server_root
                        .as_ref()
                        .expect("server root should be provided when enabling whitelist"),
                )
                .await
                {
                    Ok(whitelist) => Some(Arc::new(whitelist)),
                    Err(WhitelistParseError::IO(err)) => {
                        println!(
                            "\x1b[38;5;11mCritical: Couldn't read whitelist. Got err: {err}\x1b[0m"
                        );
                        std::process::exit(1);
                    }
                    Err(WhitelistParseError::ParseJson(err)) => {
                        println!("\x1b[38;5;11mCritical: Whitelist conatined invalid JSON. Got err: {err}\x1b[0m");
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            println!(
                "\n\x1b[38;2;0;200;0mSpoofer listening on port {}\x1b[0m\n",
                args.port
            );

            let (start_sender, mut start_reciever) = tokio::sync::mpsc::channel::<()>(1);

            // We handle connections and loop until we recieve a Login request
            loop {
                if tokio::select!(
                    Ok((stream, address)) = listener.accept() => {
                        let start_sender = start_sender.clone();

                        let whitelist = whitelist.clone();

                        task::spawn(async move {
                            let address = format!("\x1b[38;5;14m{address}\x1b[0m");
                            println!("{} Connection from {}", Local::now().format(TIME_FORMAT), address);

                            let status = |message: &str| {
                                println!("{} {} → {}", Local::now().format(TIME_FORMAT), &address, message);
                            };

                            let mut codec = ServerCodec::new(stream);

                            let output = async {loop {match codec.read_packet().await? {
                                Serverbound::Generic(packet) => match packet {
                                    generic_packets::Generic::ServerListPing(_) => {
                                        status("Recieved legacy server list ping");
                                        break Ok(false)
                                    }
                                    _ => {},
                                },
                                Serverbound::V760(packet) => match packet {
                                    serverbound::V760::Status(packet) => {match packet {
                                        serverbound::StatusPacket::StatusRequest{} => {
                                            status("Requested status");
                                            codec.send_packet(clientbound::StatusPacket::StatusResponse{ json_response: String::from(STATUS_RESPONSE) }).await?;
                                            status("Sent status");
                                        },
                                        serverbound::StatusPacket::PingRequest{ payload } => {
                                            status("Requested ping");
                                            codec.send_packet(clientbound::StatusPacket::PingResponse{ payload }).await?;
                                            status("Sent pong");
                                            break Ok(false)
                                        },
                                    }},
                                    serverbound::V760::Login(packet) => {match packet {
                                        serverbound::LoginPacket::LoginStart { name, sig_data: _, player_uuid } => {
                                            status(&format!(
                                                "Recieved login request from \x1b[38;5;14m{name}\x1b[0m{}",
                                                if let Some(uuid) = player_uuid {
                                                    format!(" with uuid: \x1b[38;5;14m{uuid:x}\x1b[0m")
                                                } else {
                                                    "".to_owned()
                                                }
                                            ));

                                            if let Some(ref whitelist) = whitelist {
                                                if let Some(uuid) = player_uuid {
                                                    if whitelist.contains(&uuid) {
                                                        codec.send_packet(clientbound::LoginPacket::Disconnect { reason: String::from(LOGIN_RESPONSE) }).await?;
                                                        status(&format!("\x1b[38;5;14m{name}\x1b[0m is whitelisted. Disconnected player"));
                                                        break Ok(true)
                                                    } else {
                                                        codec.send_packet(clientbound::LoginPacket::Disconnect {
                                                            reason: r#"{"text": "You are not whitelisted on this server"}"#.to_owned()
                                                        }).await?;
                                                        status(&format!("\x1b[38;5;14m{name}\x1b[0m is not whitelsited. Disconnected player"));
                                                    }
                                                } else {
                                                    status("Client did not provide a uuid: Can not check against whitelist");
                                                    codec.send_packet(clientbound::LoginPacket::Disconnect {
                                                        reason: r#"{"text": "You are not whitelisted on this server"}"#.to_owned()
                                                    }).await?;
                                                }
                                            } else {
                                                codec.send_packet(clientbound::LoginPacket::Disconnect { reason: String::from(LOGIN_RESPONSE) }).await?;
                                                status("Disconnected player");
                                                break Ok(true)
                                            }
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
                                    println!("{} Closed connection to {address}", Local::now().format(TIME_FORMAT));
                                    if should_we_start {
                                        start_sender.send(()).await.expect("channel shouldn't close");
                                    }
                                },
                                Err(err) => {
                                    println!("{} Killed connection to {address} on error: {err}", Local::now().format(TIME_FORMAT));
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
                        let line = line.expect("channel shouldn't close");
                        if &line == "stop\n" {
                            std::process::exit(0);
                        } else if &line == "start\n" {
                            true
                        } else {
                            println!("\x1b[38;5;11mUnknown command\x1b[0m");
                            false
                        }
                    }
                ) {
                    // We exit the connection-handling loop whenever one of the branches returns true
                    // and switch to the next state in the main loop (running the server)
                    break;
                }
            }
        }
        {
            println!("\n\x1b[38;2;0;200;0mStarting minecraft server as child process\x1b[0m\n");

            let mut mc_server = Command::new("/bin/bash")
                .args([args.start_script.as_os_str()])
                .stdin(Stdio::piped())
                .spawn()
                .expect("failed to start server in subprocess");

            let mut mc_stdin = mc_server
                .stdin
                .take()
                .expect("should have been able to bind to minecraft server stdin");

            let mut last_activity = Instant::now();
            let mut number_of_nulls: u32 = 0;

            loop {
                tokio::select!(
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
                )
            }
        }
    }
}

enum WhitelistParseError {
    ParseJson(serde_json::Error),
    IO(io::Error),
}

impl From<io::Error> for WhitelistParseError {
    fn from(err: io::Error) -> Self {
        WhitelistParseError::IO(err)
    }
}

async fn parse_whitelist(root_folder: &PathBuf) -> Result<Vec<u128>, WhitelistParseError> {
    let mut whitelist_path = PathBuf::from(root_folder);
    whitelist_path.push("whitelist.json");

    let mut ops_path = PathBuf::from(root_folder);
    ops_path.push("ops.json");

    // let mut ops_file = PathBuf::from(root_folder);
    // ops_file.push("ops.json");

    let mut whitelist = get_uuids_from_json(&whitelist_path).await?;
    whitelist.append(&mut get_uuids_from_json(&ops_path).await?);

    whitelist.sort_unstable();
    whitelist.dedup();

    Ok(whitelist)
}

async fn get_uuids_from_json(file_path: &Path) -> Result<Vec<u128>, WhitelistParseError> {
    let mut file_content = String::new();
    fs::File::open(file_path)
        .await?
        .read_to_string(&mut file_content)
        .await?;

    let objects: Vec<serde_json::Value> = match serde_json::from_str(&file_content) {
        Ok(parsed) => parsed,
        Err(err) => return Err(WhitelistParseError::ParseJson(err)),
    };

    let mut uuids = Vec::<u128>::with_capacity(objects.len());

    for entry in objects {
        match u128::from_str_radix(&entry["uuid"].to_string().replace(['-', '"'], ""), 16) {
            Ok(uuid) => uuids.push(uuid),
            Err(_) => println!(
                "\x1b[38;5;11mWarning: couldn't parse {} because of this entry:\n{entry}\x1b[0m",
                file_path.display()
            ),
        };
    }

    Ok(uuids)
}

async fn write_line(stdin: &mut tokio::process::ChildStdin, line: &str) -> io::Result<()> {
    stdin.write_all(line.as_bytes()).await?;
    stdin.flush().await
}

enum PlayercountError {
    GotNull,
    Inbound,
    IO(io::Error),
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

    LengthPrefixed::from_mc_protocol(generic_packets::HandshakePacket {
        protocol_version: McVarint::from(760_i32),
        server_address: "asd".to_owned(),
        server_port: 25561,
        next_state: generic_packets::NextState::Status,
    })
    .await?
    .serialize_write(&mut writer)
    .await?;

    writer.flush().await?;

    LengthPrefixed::from_mc_protocol(serverbound::StatusPacket::StatusRequest {})
        .await?
        .serialize_write(&mut writer)
        .await?;

    writer.flush().await?;

    let packet = {
        let mut packet_reader = get_length_prefixed_reader(&mut reader)
            .await
            .map_err(|_| PlayercountError::Inbound)?;
        clientbound::StatusPacket::deserialize_read(&mut packet_reader)
            .await
            .map_err(|_| PlayercountError::Inbound)?
    };

    if let clientbound::StatusPacket::StatusResponse { json_response } = packet {
        Ok(serde_json::from_str::<serde_json::Value>(&json_response)
            .map_err(|_| PlayercountError::Inbound)?["players"]["online"]
            .as_u64()
            .ok_or(PlayercountError::GotNull)?)
    } else {
        Err(PlayercountError::Inbound)
    }
}
