use crate::mc_protocol::McProtocol;

use super::mc_varint;
use async_trait::async_trait;
use tokio::io::{self, AsyncWriteExt};

use tokio::io::AsyncReadExt;
use std::marker::{Unpin, Send};

#[async_trait]
impl McProtocol for String {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        let bytes = self.as_bytes();
        // writer.write_all(&[0]).await?;
        todo!()
    }
    
    async fn deserialize_from_reader<R>(reader: &mut R) -> io::Result<Self> 
    where
    Self: std::marker::Sized,
    R: io::AsyncRead + Unpin + Send
    {
        let length = mc_varint::from_reader(reader).await?;
        
        let mut body = reader.take(length as u64);
        let mut output = String::new();
        
        if body.read_to_string(&mut output).await? == length as usize {
            Ok(output)
        } else {
            Err(io::Error::from(io::ErrorKind::UnexpectedEof))
        }
    }
}