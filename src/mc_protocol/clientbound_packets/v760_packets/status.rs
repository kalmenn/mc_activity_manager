use tokio::io::{self, AsyncWriteExt, AsyncReadExt};

#[derive(Debug)]
pub enum StatusPacket {
    StatusResponse {
        json_response: String,
    },
    PingResponse {
        payload: i64,
    },
}

#[async_trait::async_trait]
impl crate::mc_protocol::McProtocol for StatusPacket {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        match self {
            StatusPacket::StatusResponse { json_response } => {
                writer.write_u8(0).await?;
                // Not sure what the difference with write_all_buf() is
                json_response.serialize_write(writer).await?;
            },
            StatusPacket::PingResponse { payload } => {
                writer.write_u8(1).await?;
                writer.write_i64(*payload).await?;
            },
        }

        Ok(())
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        match reader.read_u8().await? {
            0 => {
                let json_response = String::deserialize_read(reader).await?;
                Ok(StatusPacket::StatusResponse { json_response })
            },
            1 => {
                let payload = reader.read_i64().await?;
                Ok(StatusPacket::PingResponse { payload })
            },
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected packet ID: {other}")
            )),
        }
    }
}