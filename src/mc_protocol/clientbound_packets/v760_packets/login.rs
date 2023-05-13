use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub enum LoginPacket {
    Disconnect { reason: String },
}

#[async_trait::async_trait]
impl crate::mc_protocol::McProtocol for LoginPacket {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        match self {
            Self::Disconnect { reason } => {
                writer.write_u8(0).await?;
                reason.serialize_write(writer).await?
            }
        }
        Ok(())
    }
    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self>
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send,
    {
        match reader.read_u8().await? {
            0 => Ok(Self::Disconnect {
                reason: String::deserialize_read(reader).await?,
            }),
            1 => Err(io::Error::new(
                io::ErrorKind::Other,
                "Deserializing encryption request packet is not supported",
            )),
            2 => Err(io::Error::new(
                io::ErrorKind::Other,
                "Deserializing login success packet is not supported",
            )),
            3 => Err(io::Error::new(
                io::ErrorKind::Other,
                "Deserializing set compression packet is not supported",
            )),
            4 => Err(io::Error::new(
                io::ErrorKind::Other,
                "Deserializing login plugin request packet is not supported",
            )),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected packet ID: {other}"),
            )),
        }
    }
}
