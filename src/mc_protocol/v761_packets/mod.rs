pub mod serverbound;
pub mod clientbound;

pub use clientbound::ClientboundPacket;
pub use serverbound::ServerboundPacket;

use crate::mc_protocol::{self, ConnectionState, Role, ConnectionStateLevelDeserialize};

use tokio::io;

#[derive(Debug)]
pub enum V761Packet {
    ClientboundPacket(ClientboundPacket),
    ServerboundPacket(ServerboundPacket),
}

#[async_trait::async_trait]
impl mc_protocol::RoleLevelDeserialize for V761Packet {
    async fn deserialize_read<R>(reader: &mut R, connection_state: &ConnectionState, role: &Role) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        Ok(match role {
            Role::Client => Self::ClientboundPacket(ClientboundPacket::deserialize_read(reader, connection_state).await?),
            Role::Server => Self::ServerboundPacket(ServerboundPacket::deserialize_read(reader, connection_state).await?),
        })
    }
}