mod varint;
mod codec;

use tokio::{
    task,
    io, 
    net::{TcpStream, TcpListener}
};
use futures::{stream::FuturesUnordered, StreamExt};

use varint::into_varint;
use codec::Codec;

enum Request {
    Status,
    Start
}

pub async fn wait_for_start_request() {
    // let listener = tokio::net::TcpListener::bind("127.0.0.1:6969");
    let listener = TcpListener::bind("127.0.0.1:6969")
        .await
        .expect("Couldn't bind to TCP socket");

    println!("\n\x1b[38;2;0;200;0mSpoofer listening on port 6969\x1b[0m\n");

    let mut futures = FuturesUnordered::new();
    loop {
        tokio::select! {
            Ok((stream, address)) = listener.accept() => {
                let address = format!("\x1b[38;5;14m{}\x1b[0m", address);
                
                futures.push(task::spawn(async move {
                    println!("Connection from {}", address);
                    let result = handle_connection(stream).await;
                    match &result {
                        Ok(_) => {
                            println!("Closed connection to {address}");
                        },
                        Err(err) => {
                            println!("Killed connection to {address} on error: {err}");
                        }
                    }
                    result
                }));
            },
            Some(request) = futures.next() => {
                if let Ok(Ok(Request::Start)) = request {
                    break
                }
            }
        }
    }
}

enum RequestState {
    Handshake,
    Login,
    Status
}

async fn handle_connection(stream: TcpStream) -> io::Result<Request>{
    let address = format!("\x1b[38;5;14m{}\x1b[0m", &stream.peer_addr()?);

    let status = |status: &str| {
        println!("{} → {}", address, status);
    };

    let mut request_state = RequestState::Handshake;

    let mut codec = Codec::new(stream)?;
    loop {
        let message = codec.read_message().await?;

        match request_state {
            RequestState::Handshake => {
                status("State: Handshaking");
                match &message.iter().last() {
                    Some(1) => {
                        request_state = RequestState::Status;
                        status("Switching state to: Status");
                    },
                    Some(2) => {
                        request_state = RequestState::Login;
                        status("Switching state to: Login");
                    }
                    _ => {
                        status("Garbled packet");
                        return Err(io::Error::from(io::ErrorKind::InvalidData));
                    }
                }
            },
            RequestState::Status => {
                status("State: Status");
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