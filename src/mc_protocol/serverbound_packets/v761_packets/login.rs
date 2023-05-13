use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub enum LoginPacket {
    LoginStart {
        name: String,
        player_uuid: Option<u128>,
    },
}

#[async_trait::async_trait]
impl crate::mc_protocol::McProtocol for LoginPacket {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        match self {
            Self::LoginStart { name, player_uuid } => {
                if name.len() > 16 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Player name can't be over 16 characters long",
                    ));
                };
                name.serialize_write(writer).await?;
                if let Some(uuid) = player_uuid {
                    writer.write_u8(1).await?;
                    writer.write_u128(*uuid).await?
                } else {
                    writer.write_u8(0).await?;
                }
                Ok(())
            }
        }
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self>
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send,
    {
        match reader.read_u8().await? {
            0 => {
                let name = String::deserialize_read(reader).await?;
                if name.len() > 16 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Player name can't be over 16 characters long",
                    ));
                }
                let player_uuid = match reader.read_u8().await? {
                    0 => None,
                    1 => Some(reader.read_u128().await?),
                    other => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unexpected boolean variant: {other}"),
                        ))
                    }
                };
                Ok(LoginPacket::LoginStart { name, player_uuid })
            }
            1 => Err(io::Error::new(
                io::ErrorKind::Other,
                "Deserializing encryption response packets is not supported",
            )),
            2 => Err(io::Error::new(
                io::ErrorKind::Other,
                "Deserializing login plugin response packet is not supported",
            )),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected packet ID: {other}"),
            )),
        }
    }
}
