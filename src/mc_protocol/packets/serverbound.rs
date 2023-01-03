use crate::mc_protocol::{
    McProtocol,
    data_types::mc_varint,
};

use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use std::marker::{Unpin, Send};

pub struct HandshakePacket{
    protocol_version: u32,
    server_address: String,
    server_port: u16,
    next_state: NextState,
}

enum NextState {
    Login,
    Status
}

#[async_trait]
impl McProtocol for HandshakePacket {
    async fn deserialize_from_reader<R>(reader: &mut R) -> io::Result<Self> 
    where
        R: io::AsyncRead + Unpin + Send
    {
        let protocol_version = mc_varint::from_reader(reader).await?;
        let server_address = String::deserialize_from_reader(reader).await?;
        let server_port = reader.read_u16().await?;
        let next_state = match reader.read_u8().await? {
            1 => NextState::Login,
            2 => NextState::Status,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "next_state field wasn't a valid enum variant")),
        };

        Ok(HandshakePacket { 
            protocol_version, 
            server_address,
            server_port,
            next_state 
        })
    }

    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        writer.write_all(&mc_varint::into_varint(self.protocol_version)).await?;
        if self.server_address.chars().count() > 255 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "server_address can't be over 255 characters long"))
        }
        self.server_address.serialize_write(writer).await?;
        writer.write_u16(self.server_port).await?;
        writer.write_u8(match self.next_state {
            NextState::Login => 1,
            NextState::Status => 2,
        }).await?;

        writer.flush().await?;

        Ok(())
    }
}