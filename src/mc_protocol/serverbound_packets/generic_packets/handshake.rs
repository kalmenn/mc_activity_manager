use crate::mc_protocol::data_types::McVarint;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub struct HandshakePacket {
    pub protocol_version: McVarint,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: NextState,
}

#[derive(Debug)]
pub enum NextState {
    Login,
    Status,
}

impl std::fmt::Display for NextState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Login => "Login",
                Self::Status => "Status",
            }
        )
    }
}

#[async_trait::async_trait]
impl crate::mc_protocol::McProtocol for HandshakePacket {
    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::AsyncRead + Unpin + Send,
    {
        {
            let packet_id = reader.read_u8().await?;
            if packet_id != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected packet ID: {packet_id}"),
                ));
            }
        }
        let protocol_version = McVarint::deserialize_read(reader).await?;
        let server_address = String::deserialize_read(reader).await?;
        let server_port = reader.read_u16().await?;
        let next_state = match reader.read_u8().await? {
            1 => NextState::Status,
            2 => NextState::Login,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "next_state field wasn't a valid enum variant",
                ))
            }
        };

        Ok(HandshakePacket {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }

    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        writer.write_u8(0).await?;
        self.protocol_version.serialize_write(writer).await?;
        if self.server_address.as_bytes().len() > 255 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "server_address can't be over 255 bytes long",
            ));
        }
        self.server_address.serialize_write(writer).await?;
        writer.write_u16(self.server_port).await?;
        writer
            .write_u8(match self.next_state {
                NextState::Status => 1,
                NextState::Login => 2,
            })
            .await?;

        Ok(())
    }
}
