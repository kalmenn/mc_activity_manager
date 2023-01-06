pub mod data_types;
pub mod v760_packets;
pub mod v761_packets;
pub mod generic_packets;

mod codec;
pub use codec::Codec;

use tokio::io;
use std::marker::{Unpin, Send};

/// Something is McProtocol if it can serialize / deserialize itself
/// according to the minecraft server protocol
#[async_trait::async_trait]  
pub trait McProtocol {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    ;
    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    ;
}

#[async_trait::async_trait]
pub trait RoleLevelDeserialize {
    async fn deserialize_read<R>(reader: &mut R, connection_state: &ConnectionState, role: &Role) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    ;
}

#[async_trait::async_trait]
pub trait ConnectionStateLevelDeserialize {
    async fn deserialize_read<R>(reader: &mut R, connection_state: &ConnectionState) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    ;
}

/// Encodes the currently supported protocol versions
#[derive(Debug)]
pub enum ProtocolVersion {
    V760,
    V761,
}

impl TryFrom<i32> for ProtocolVersion {
    type Error = io::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            760 => Ok(Self::V760),
            761 => Ok(Self::V761),
            other => Err(Self::Error::new(
                io::ErrorKind::Other,
                format!("protocol version {other} not supported")
            )),
        }
    }
}

/// Encodes the role of the codec in the connection.
#[derive(Debug)]
pub enum Role {
    Server,
    Client,
}

#[derive(Debug)]
pub enum ConnectionState {
    Handshaking,
    Status,
    Login,
}

use self::v760_packets::V760Packet;
use self::v761_packets::V761Packet;
use self::generic_packets::GenericPacket;

#[derive(Debug)]
pub enum Packet {
    V760(V760Packet),
    V761(V761Packet),
    Generic(GenericPacket),
}

impl Packet {
    pub async fn deserialize_read<R>(reader: &mut R, connection_state: &ConnectionState, role: &Role, protocol_version: &Option<ProtocolVersion>) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        if let Some(protocol_version) = protocol_version {
            match protocol_version {
                ProtocolVersion::V760 => Ok(Self::V760(V760Packet::deserialize_read(reader, connection_state, role).await?)),
                ProtocolVersion::V761 => Ok(Packet::V761(V761Packet::deserialize_read(reader, connection_state, role).await?)),
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The protocol version must be known when deserializing with Packet::deserializing_read()"
            ))
        }
    }

    pub fn get_protocol_version(&self) -> Option<ProtocolVersion> {
        match self {
            Self::V760(_) => Some(ProtocolVersion::V760),
            Self::V761(_) => Some(ProtocolVersion::V761),
            Self::Generic(_) => None
        }
    }
}