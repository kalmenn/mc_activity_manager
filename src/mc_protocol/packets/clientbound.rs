use crate::mc_protocol::{
    McProtocol,
};

use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use std::marker::{Unpin, Send};

pub trait ClientboundPacket: McProtocol {}

pub enum StatusPacket {
    StatusResponse {
        json_response: String,
    },
    PingResponse {
        payload: i64,
    },
}

impl ClientboundPacket for StatusPacket {}

#[async_trait]
impl McProtocol for StatusPacket {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        match self {
            StatusPacket::StatusResponse { json_response } => {
                writer.write_u8(0).await?;
                // Not sure what the difference with write_all_buf() is
                writer.write_all(json_response.as_bytes()).await?;
            },
            StatusPacket::PingResponse { payload } => {
                writer.write_u8(1).await?;
                writer.write_i64(*payload).await?;
            },
        }
        writer.flush().await
    }
    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        match reader.read_u8().await? {
            0 => {
                let mut json_response = String::deserialize_read(reader).await?;
                reader.read_to_string(&mut json_response).await?;
                Ok(StatusPacket::StatusResponse { json_response })
            },
            1 => {
                let payload = reader.read_i64().await?;
                Ok(StatusPacket::PingResponse { payload })
            },
            packet_id => Err(io::Error::new(io::ErrorKind::InvalidData, format!("unexpected packet ID: {packet_id}"))),
        }
    }
}