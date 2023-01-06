use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use crate::mc_protocol::{data_types::McVarint, McProtocol};

pub struct LengthPrefixed {
    data: Vec<u8>,
}

#[async_trait::async_trait]
impl McProtocol for LengthPrefixed {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        McVarint::from(match i32::try_from(self.data.len()) {
            Ok(value) => value,
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Object length was outside of i32 bounds: {:?}", self.data)
            ))
        }).serialize_write(writer).await?;

        writer.write_all(&self.data).await?;

        Ok(())
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        let length: u32 = McVarint::deserialize_read(reader).await?.try_into()?;

        let mut adapter = reader.take(length.into());
        let mut data = Vec::new();

        let bytes_read = adapter.read_to_end(&mut data).await?;

        if bytes_read != usize::try_from(length).expect("u32 should always be within the bounds of usize") {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "couldn't read all the bytes from length-prefixed value"
            ));
        };

        Ok(Self { data })
    }
}

impl From<Vec<u8>> for LengthPrefixed {
    fn from(data: Vec<u8>) -> Self {
        Self{ data }
    }
}

impl From<LengthPrefixed> for Vec<u8> {
    fn from(length_prefixed: LengthPrefixed) -> Self {
        length_prefixed.data
    }
}