pub mod generic_packets;
pub mod v760_packets;
pub mod v761_packets;

use generic_packets::Generic;
use v760_packets::V760;
use v761_packets::V761;

use crate::mc_protocol::{
    ConnectionState,
    ProtocolVersion,
    ProtocolVersionLevelDeserialize,
    ConnectionStateLevelDeserialize,
};
use tokio::io;

/// A minecraft server packet sent from a client to a server
pub enum Serverbound {
    V760(V760),
    V761(V761),
    Generic(Generic),
}

#[async_trait::async_trait]
impl ProtocolVersionLevelDeserialize for Serverbound {
    async fn deserialize_read<R>(reader: &mut R, connection_state: ConnectionState, protocol_version: ProtocolVersion) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        Ok(match protocol_version {
            ProtocolVersion::V760 => Self::V760(V760::deserialize_read(reader, connection_state).await?),
            ProtocolVersion::V761 => Self::V761(V761::deserialize_read(reader, connection_state).await?),
        })
    }
}

impl From<Serverbound> for Option<ProtocolVersion> {
    fn from(packet: Serverbound) -> Self {
        match packet {
            Serverbound::V760(_) => Some(ProtocolVersion::V760),
            Serverbound::V761(_) => Some(ProtocolVersion::V761),
            Serverbound::Generic(_) => None,
        }
    }
}