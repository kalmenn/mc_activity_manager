use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub enum StatusPacket {
    StatusRequest {},
    PingRequest {
        payload: i64,
    }
}

#[async_trait::async_trait]
impl crate::mc_protocol::McProtocol for StatusPacket {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        match self {
            Self::StatusRequest{} => writer.write_u8(0).await?,
            Self::PingRequest{payload} => {
                writer.write_u8(1).await?;
                writer.write_i64(*payload).await?;
            },
        };

        Ok(())
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        match reader.read_u8().await? {
            0 => Ok(Self::StatusRequest{}),
            1 => Ok(Self::PingRequest { payload: reader.read_i64().await? }),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected packet ID: {other}")
            )),
        }
    }
}