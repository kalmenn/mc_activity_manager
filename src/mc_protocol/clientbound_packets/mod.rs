pub mod v760_packets;
pub mod v761_packets;

use v760_packets::V760;
use v761_packets::V761;

use crate::mc_protocol::{
    ConnectionState, ConnectionStateLevelDeserialize, ProtocolVersion,
    ProtocolVersionLevelDeserialize,
};
use tokio::io;

/// A minecraft server packet sent from a server to a client
pub enum Clientbound {
    V760(V760),
    V761(V761),
}

#[async_trait::async_trait]
impl ProtocolVersionLevelDeserialize for Clientbound {
    async fn deserialize_read<R>(
        reader: &mut R,
        connection_state: ConnectionState,
        protocol_version: ProtocolVersion,
    ) -> io::Result<Self>
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send,
    {
        Ok(match protocol_version {
            ProtocolVersion::V760 => {
                Self::V760(V760::deserialize_read(reader, connection_state).await?)
            }
            ProtocolVersion::V761 => {
                Self::V761(V761::deserialize_read(reader, connection_state).await?)
            }
        })
    }
}
