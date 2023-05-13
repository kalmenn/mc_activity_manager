use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use crate::mc_protocol::data_types::{LengthPrefixed, McVarint};

// https://wiki.vg/index.php?title=Protocol&oldid=17873#Login_Start

#[derive(Debug)]
pub enum LoginPacket {
    LoginStart {
        name: String,
        sig_data: Option<SigData>,
        player_uuid: Option<u128>,
    },
}

pub struct SigData {
    timestamp: i64,
    public_key: Vec<u8>,
    signature: Vec<u8>,
}

impl std::fmt::Debug for SigData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigData")
            .field("timestamp", &self.timestamp)
            .field("public_key", &format!("{:x?}", self.public_key))
            .field("signature", &format!("{:x?}", self.signature))
            .finish()
    }
}

#[async_trait::async_trait]
impl crate::mc_protocol::McProtocol for LoginPacket {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        match self {
            Self::LoginStart {
                name,
                sig_data,
                player_uuid,
            } => {
                if name.len() > 16 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Player name can't be over 16 characters long",
                    ));
                };
                name.serialize_write(writer).await?;

                if let Some(sig_data) = sig_data {
                    writer.write_u8(1).await?;

                    writer.write_i64(sig_data.timestamp).await?;

                    McVarint::from(match i32::try_from(sig_data.public_key.len()) {
                        Ok(value) => value,
                        Err(_) => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "public key length was outside of i32 bounds",
                            ))
                        }
                    })
                    .serialize_write(writer)
                    .await?;
                    writer.write_all(&sig_data.public_key).await?;

                    McVarint::from(match i32::try_from(sig_data.signature.len()) {
                        Ok(value) => value,
                        Err(_) => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "signature length was outside of i32 bounds",
                            ))
                        }
                    })
                    .serialize_write(writer)
                    .await?;
                    writer.write_all(&sig_data.signature).await?;
                } else {
                    writer.write_u8(0).await?;
                }

                if let Some(uuid) = player_uuid {
                    writer.write_u8(1).await?;
                    writer.write_u128(*uuid).await?
                } else {
                    writer.write_u8(0).await?;
                }
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
            0 => {
                let name = String::deserialize_read(reader).await?;
                if name.len() > 16 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Player name can't be over 16 characters long",
                    ));
                }
                let sig_data = match reader.read_u8().await? {
                    0 => None,
                    1 => {
                        let timestamp = reader.read_i64().await?;
                        let public_key: Vec<u8> =
                            LengthPrefixed::deserialize_read(reader).await?.into();
                        let signature: Vec<u8> =
                            LengthPrefixed::deserialize_read(reader).await?.into();
                        Some(SigData {
                            timestamp,
                            public_key,
                            signature,
                        })
                    }
                    other => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unexpected boolean variant: {other}"),
                        ))
                    }
                };
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
                Ok(LoginPacket::LoginStart {
                    name,
                    sig_data,
                    player_uuid,
                })
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
