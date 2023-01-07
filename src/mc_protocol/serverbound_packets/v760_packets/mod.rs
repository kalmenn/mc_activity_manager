mod status;
pub use status::StatusPacket;

mod login;
pub use login::LoginPacket;

#[derive(Debug)]
pub enum V760 {
    Handshake(HandshakePacket),
    Status(StatusPacket),
    Login(LoginPacket),
}

use crate::mc_protocol::{
    self,
    ConnectionState,
    McProtocol,
    serverbound_packets::generic_packets::HandshakePacket
};
use tokio::io;

#[async_trait::async_trait]
impl mc_protocol::ConnectionStateLevelDeserialize for V760 {
    async fn deserialize_read<R>(reader: &mut R, connection_state: ConnectionState) -> io::Result<Self> 
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