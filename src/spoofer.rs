use std::net::SocketAddr;

use tokio::{
    task::{self, JoinHandle},
    sync::mpsc::{self, Sender, Receiver},
    io,
    net::{TcpStream, TcpListener}
};

use crate::mc_protocol::data_types::varint::into_varint;
use crate::mc_protocol::codec::Codec;

/// Encodes the state of a connection being handled by a [`Spoofer`]
enum RequestState {
    Handshake,
    Login,
    Status
}

/// Encodes different types of requests handled by [`spoofers`](Spoofer).
#[derive(Debug)]
enum Request {
    Status,
    Start
}

/// A spoofer pretends to be a minecraft server by listening for
/// incoming connections, and serving custom data.
/// 
/// Build a `Spoofer` with the [`new`](Self::new()) method and appending
/// build methods like [`set_port`](Self::set_port()) or [`set_debug`](Self::set_debug())
/// 
/// A `Spoofer` needs to be told to start listening after it has been built.
/// Do this with [`start_listening`][Self::start_listening()]
pub struct Spoofer {
    /// Which socket to listen on
    socket: SocketAddr,
    /// Sets the verbosity of status messages in the terminal
    show_debug: bool,
    /// Incoming requests are sent through this channel to be dealt with by
    /// another task.
    request_channel: (Sender<Request>, Receiver<Request>),
    /// Holds the task that serves clients if there is any.
    /// Can be created with [`start_listening()`](Self::start_listening())
    listener: Option<JoinHandle<()>>
}

impl Spoofer {

    /// Creates a new spoofer with default settings.
    /// 
    /// By default, the spoofer listens on local port 25565.
    pub fn new() -> Spoofer {
        Spoofer {
            socket: "127.0.0.1:25565".parse::<SocketAddr>().unwrap(),
            show_debug: false,
            request_channel: mpsc::channel(10),
            listener: None
        }
    }

    /// To be used when constructing a `Spoofer`.
    /// 
    /// Sets the port to listen on to `port`.
    pub fn set_port(mut self, port: u16) -> Self {
        self.socket.set_port(port);
        self
    }

    /// To be used when constructing a `Spoofer`.
    /// 
    /// Sets the verbosity of status messages.
    pub fn set_debug(mut self, show_debug: bool) -> Self {
        self.show_debug = show_debug;
        self
    }

    /// Start serving clients with the settings provided during the 
    /// building of the spoofer.
    /// 
    /// Sends all incoming [requests](Request) through buffered a channel
    /// for evaluation.
    pub fn start_listening(&mut self) {
        if self.listener.is_some() {
            panic!("Spoofer is already listening in another task");
        }

        let tx = self.request_channel.0.clone();
        let socket = self.socket;

        let show_debug = self.show_debug;

        self.listener = Some(task::spawn(async move { 
            let listener = TcpListener::bind(socket)
            .await
            .expect("Couldn't bind to TCP socket");
            
            loop {
                let tx = tx.clone();
                if let Ok((stream, address)) = listener.accept().await  {
                    let address = format!("\x1b[38;5;14m{}\x1b[0m", address);
                    println!("Connection from {}", address);
                    
                    task::spawn(async move {
                        match handle_connection(stream, show_debug).await {
                            Ok(request) => {
                                println!("Closed connection to {address}");
                                tx.send(request).await.expect("Reciever of requests dropped");
                            },
                            Err(err) => {
                                println!("Killed connection to {address} on error: {err}");
                            }
                        }
                    });
                }
            }
        }));

        println!("\n\x1b[38;2;0;200;0mSpoofer listening on port {}\x1b[0m\n", self.socket.port());
    }

    /// Reads from the request channel and returns only 
    /// when a start request has been recieved
    pub async fn wait_for_start_request(&mut self) {
        loop {
            if let Some(Request::Start) = self.request_channel.1.recv().await {
                break
            }
        }
    }

}

impl Drop for Spoofer {
    fn drop(&mut self) {
        if let Some(task) = self.listener.take() {
            task.abort();
        }
    }
}

/// Takes ownership of the incoming stream and returns the [kind of request](Request)
/// the client sent
async fn handle_connection(stream: TcpStream, show_debug: bool) -> io::Result<Request>{
    let address = format!("\x1b[38;5;14m{}\x1b[0m", &stream.peer_addr()?);

    let status = |message: &str| {
        println!("{} → {}", address, message);
    };

    let debug = |message: &str| {
        if show_debug {
            status(message);
        }
    };

    let mut request_state = RequestState::Handshake;

    let mut codec = Codec::new(stream)?;
    loop {
        let message = codec.read_message().await?;

        match request_state {
            RequestState::Handshake => {
                debug("State: Handshaking");
                match &message.iter().last() {
                    Some(1) => {
                        request_state = RequestState::Status;
                        debug("Switching state to: Status");
                    },
                    Some(2) => {
                        request_state = RequestState::Login;
                        debug("Switching state to: Login");
                    }
                    _ => {
                        status("Garbled packet");
                        return Err(io::Error::from(io::ErrorKind::InvalidData));
                    }
                }
            },
            RequestState::Status => {
                debug("State: Status");
                match &message[0] {
                    0 => {
                        status("Requested status");
                        let mut text = "{\"description\":[{\"text\":\"Hors Ligne ...\n\",\"color\":\"gold\"},{\"text\":\"Connectez vous pour démarrer le serveur\",\"color\":\"dark_green\"}],\"players\":{\"max\":0,\"online\":1,\"sample\":[{\"name\":\"J'ai pas hacké je jure\",\"id\":\"4566e69f-c907-48ee-8d71-d7ba5aa00d20\"}]},\"version\":{\"name\":\"1.19.2\",\"protocol\":760}}"
                        .as_bytes().to_vec();

                        let mut response = Vec::<u8>::new();
                        response.push(0);
                        response.append(&mut into_varint(text.len()));
                        response.append(&mut text);

                        codec.send_message(response).await?;
                        status("sent status");
                    }
                    1 => {
                        status("Requested ping");
                        codec.send_message(message).await?;
                        status("Sent pong");
                        return Ok(Request::Status);
                    }
                    _ => {
                        status("Garbled packet");
                        return Err(io::Error::from(io::ErrorKind::InvalidData));
                    }
                }
            }
            RequestState::Login => {
                status("Requested login");
                let mut text = "[{\"text\":\"Serveur Hors Ligne\n\n\",\"color\":\"red\"},{\"text\":\"Demande de démarrage reçue,\nle serveur devrait être disponible d'ici une minute\",\"color\":\"white\"}]"
                .as_bytes().to_vec();

                let mut response = Vec::<u8>::new();
                response.push(0);
                response.append(&mut into_varint(text.len()));
                response.append(&mut text);

                codec.send_message(response).await?;
                status("Sent disconnect message");

                return Ok(Request::Start);
            }
        }
    }
}