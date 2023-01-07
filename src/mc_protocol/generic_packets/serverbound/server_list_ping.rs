use tokio::{
    io::{self, AsyncReadExt},
    net::tcp::OwnedReadHalf,
};
use crate::mc_protocol::McProtocol;

/// Data that is constant across all server list ping packets
const STATIC_HEADER: [u8; 27] = [0xfe, 0x01, 0xfa, 0x00, 0x0b, 0x00, 0x4D, 0x00, 0x43, 0x00, 0x7C, 0x00, 0x50, 0x00, 0x69, 0x00, 0x6E, 0x00, 0x67, 0x00, 0x48, 0x00, 0x6F, 0x00, 0x73, 0x00, 0x74];

#[derive(Debug)]
#[allow(dead_code)]
pub struct ServerListPingPacket {
    protocol_version: u8,
    server_address: String,
    server_port: i32
}

/// Will call peek on the read half and attempt to read 3 bytes from the stream.
/// If they match the first bytes sent by a server list ping, returns true.
/// 
/// # Errors
/// Returns an error if it can't read from the reader or if the number of bytes read differs from 3
pub async fn is_packet_server_list_ping<R>(reader: &mut OwnedReadHalf) -> io::Result<bool> {
    let mut peeked_bytes = [0u8; 3];
    if reader.peek(&mut peeked_bytes).await? != 3 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "expected at least 3 bytes"
        ))
    };
    Ok(peeked_bytes == [0xfe, 0x01, 0xfa])
}

#[async_trait::async_trait]
impl McProtocol for ServerListPingPacket {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        todo!()
    }
    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        {
            let mut header = [0u8; 27];
            reader.read_exact(&mut header).await?;
            if header != STATIC_HEADER {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "not a valid server list ping packet"
                ))
            }
        }

        let hostname_length_bytes = match usize::try_from(reader.read_i16().await?) {
            Ok(value) => value,
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "packet data length was outside of usize bounds (probably negative)"
            )),
        } - 7;

        let protocol_version = reader.read_u8().await?;

        let server_address = {
            let hostname_length_chars = match usize::try_from(reader.read_i16().await?) {
                Ok(value) => value,
                Err(_) => return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "server address length in characters was outside of usize bounds (probably negative)"
                )),
            };

            let mut buffer = Vec::<u16>::with_capacity(hostname_length_bytes / 2);
            for _ in 0..(hostname_length_bytes / 2) {
                buffer.push(reader.read_u16().await?);
            }

            let server_address = match String::from_utf16(buffer.as_slice()) {
                Ok(string) => string,
                Err(_) => return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "server address wasn't valid UTF-16"
                )),
            };

            if server_address.len() != hostname_length_chars {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "server address length in characters had a different length than expected"
                ))
            }

            server_address
        };


        let server_port = reader.read_i32().await?;

        Ok(ServerListPingPacket { protocol_version, server_address, server_port })
    }
}