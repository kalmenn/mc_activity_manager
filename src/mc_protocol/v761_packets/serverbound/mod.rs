mod status;
pub use status::StatusPacket;

mod login;
pub use login::LoginPacket;

#[derive(Debug)]
pub enum ServerboundPacket {
    Handshake(HandshakePacket),
    Status(StatusPacket),
    Login(LoginPacket),
}

use crate::mc_protocol::{
    ConnectionState,
    McProtocol,
    generic_packets::serverbound::HandshakePacket
};
use tokio::io;

impl ServerboundPacket {
    pub async fn deserialize_read<R>(reader: &mut R, connection_state: &ConnectionState) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        Ok(match connection_state {
            ConnectionState::Handshaking => Self::Handshake(HandshakePacket::deserialize_read(reader).await?),
            ConnectionState::Status => Self::Status(StatusPacket::deserialize_read(reader).await?),
            ConnectionState::Login => Self::Login(LoginPacket::deserialize_read(reader).await?),
        })
    }
}