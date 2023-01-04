use crate::mc_protocol::McProtocol;
use super::McVarint;

use tokio::io::{self, AsyncWriteExt, AsyncReadExt};

use std::marker::{Unpin, Send};

#[async_trait::async_trait]
impl McProtocol for String {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        let bytes = self.as_bytes();
        McVarint::from(bytes.len() as i32).serialize_write(writer).await?;
        writer.write_all(bytes).await?;
        writer.flush().await
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        let length: u32 = match i32::from(McVarint::deserialize_read(reader).await?).try_into() {
            Ok(value) => value,
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "failed to convert string length from i32 to u32. It was probably negative"
            )),
        };

        let mut body = {
            reader.take(length.into())
        };

        let mut output = String::new();
        if body.read_to_string(&mut output).await? == length as usize {
            Ok(output)
        } else {
            Err(io::Error::from(io::ErrorKind::UnexpectedEof))
        }
    }
}