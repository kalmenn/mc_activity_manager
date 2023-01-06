pub mod data_types;
pub mod packets_761;

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

/// Encodes the currently supported protocol versions
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