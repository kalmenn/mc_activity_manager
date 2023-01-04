pub mod data_types;
pub mod packets;

mod old_codec;
pub use old_codec::Codec;

use tokio::io;
use std::marker::{Unpin, Send};

/// Something is McProtocol if it can serialize / deserialize itself
/// according to the minecraft server protocol
#[async_trait::async_trait]  
trait McProtocol {
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

pub enum ConnectionState {
    Handshaking,
    Status,
    Login,
}