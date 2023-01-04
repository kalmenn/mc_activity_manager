use crate::mc_protocol::{
    McProtocol,
    data_types::McVarint,
};

use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use std::marker::{Unpin, Send};

pub struct HandshakePacket{
    protocol_version: McVarint,
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
    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        R: io::AsyncRead + Unpin + Send
    {
        {
            let packet_id = reader.read_u8().await?;
            if packet_id != 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unexpected packet ID: {packet_id}")))
            }
        }
        let protocol_version = McVarint::deserialize_read(reader).await?;
        let server_address = String::deserialize_read(reader).await?;
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
        writer.write_u8(0).await?;
        self.protocol_version.serialize_write(writer).await?;
        if self.server_address.as_bytes().len() > 255 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "server_address can't be over 255 bytes long"))
        }
        self.server_address.serialize_write(writer).await?;
        writer.write_u16(self.server_port).await?;
        writer.write_u8(match self.next_state {
            NextState::Login => 1,
            NextState::Status => 2,
        }).await?;

        writer.flush().await
    }
}