use crate::mc_protocol::{
    McProtocol,
    data_types::McVarint,
};

use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use std::marker::{Unpin, Send};

#[derive(Debug)]
pub enum ServerboundPacket {
    Handshake(HandshakePacket),
    Status(StatusPacket),
    Login(LoginPacket),
}

#[derive(Debug)]
pub struct HandshakePacket{
    pub protocol_version: McVarint,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: NextState,
}

#[derive(Debug)]
pub enum NextState {
    Login,
    Status
}

impl std::fmt::Display for NextState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Login => "Login",
            Self::Status => "Status"
        })
    }
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
            1 => NextState::Status,
            2 => NextState::Login,
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

        Ok(())
    }
}

#[derive(Debug)]
pub enum StatusPacket {
    StatusRequest {},
    PingRequest {
        payload: i64,
    }
}

#[async_trait]
impl McProtocol for StatusPacket {
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

#[derive(Debug)]
pub enum LoginPacket {
    LoginStart{
        name: String,
        player_uuid: Option<u128>,
    }
}

#[async_trait]
impl McProtocol for LoginPacket {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        match self {
            Self::LoginStart{name, player_uuid} => {
                if name.len() > 16 {return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Player name can't be over 16 characters long"
                ))};
                name.serialize_write(writer).await?;
                if let Some(uuid) = player_uuid {
                    writer.write_u8(1).await?;
                    writer.write_u128(*uuid).await?
                } else {
                    writer.write_u8(0).await?;
                }
                Ok(())
            },
        }
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        match reader.read_u8().await? {
            0 => {
                let name = String::deserialize_read(reader).await?;
                if name.len() > 16 {return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Player name can't be over 16 characters long"
                ))}
                let player_uuid = match reader.read_u8().await? {
                    0 => None,
                    1 => Some(reader.read_u128().await?),
                    other => return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unexpected boolean variant: {other}")
                    )),
                };
                Ok(LoginPacket::LoginStart { name, player_uuid })
            },
            1 => Err(io::Error::new(
                io::ErrorKind::Other,
                "Deserializing encryption response packets is not implemented"
            )),
            2 => Err(io::Error::new(
                io::ErrorKind::Other,
                "Deserializing login plugin response packet is not implemented"
            )),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected packet ID: {other}")
            )),
        }
    }
}