use crate::mc_protocol::{data_types::LengthPrefixed, McProtocol};

use tokio::io;

use std::marker::{Send, Unpin};

#[async_trait::async_trait]
impl McProtocol for &str {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        LengthPrefixed::from(Vec::from(self.as_bytes()))
            .serialize_write(writer)
            .await
    }

    async fn deserialize_read<R>(_: &mut R) -> io::Result<Self>
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send,
    {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl McProtocol for String {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        LengthPrefixed::from(Vec::from(self.as_bytes()))
            .serialize_write(writer)
            .await
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self>
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send,
    {
        match String::from_utf8(Vec::<u8>::from(
            LengthPrefixed::deserialize_read(reader).await?,
        )) {
            Ok(string) => Ok(string),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "String wasn't valid UTF-8",
            )),
        }
    }
}
