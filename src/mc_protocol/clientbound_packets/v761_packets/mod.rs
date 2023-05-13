mod status;
pub use status::StatusPacket;

mod login;
pub use login::LoginPacket;

use crate::mc_protocol::{self, ConnectionState, McProtocol};

use tokio::io;

/// A minecraft server packet for protocol version 761 sent from a server to a client
#[derive(Debug)]
pub enum V761 {
    Status(StatusPacket),
    Login(LoginPacket),
}

#[async_trait::async_trait]
impl mc_protocol::ConnectionStateLevelDeserialize for V761 {
    async fn deserialize_read<R>(
        reader: &mut R,
        connection_state: ConnectionState,
    ) -> io::Result<Self>
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send,
    {
        match connection_state {
            ConnectionState::Handshaking => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "There is no client bound packet in the handshaking stage",
            )),
            ConnectionState::Status => {
                Ok(Self::Status(StatusPacket::deserialize_read(reader).await?))
            }
            ConnectionState::Login => Ok(Self::Login(LoginPacket::deserialize_read(reader).await?)),
        }
    }
}
