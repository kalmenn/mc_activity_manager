use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufWriter, Take};

use crate::mc_protocol::{data_types::McVarint, McProtocol};

pub struct LengthPrefixed {
    data: Vec<u8>,
}

#[async_trait::async_trait]
impl McProtocol for LengthPrefixed {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        McVarint::from(match i32::try_from(self.data.len()) {
            Ok(value) => value,
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Object length was outside of i32 bounds: {:?}", self.data),
                ))
            }
        })
        .serialize_write(writer)
        .await?;

        writer.write_all(&self.data).await?;

        Ok(())
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self>
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send,
    {
        let length: u32 = McVarint::deserialize_read(reader).await?.try_into()?;

        let mut adapter = reader.take(length.into());
        let mut data = Vec::new();

        let bytes_read = adapter.read_to_end(&mut data).await?;

        if bytes_read
            != usize::try_from(length).expect("u32 should always be within the bounds of usize")
        {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "couldn't read all the bytes from length-prefixed value",
            ));
        };

        Ok(Self { data })
    }
}

impl From<Vec<u8>> for LengthPrefixed {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl From<LengthPrefixed> for Vec<u8> {
    fn from(length_prefixed: LengthPrefixed) -> Self {
        length_prefixed.data
    }
}

impl LengthPrefixed {
    pub async fn from_mc_protocol(object: impl McProtocol) -> io::Result<Self> {
        let bytes = {
            let mut writer = BufWriter::new(Vec::<u8>::new());
            object.serialize_write(&mut writer).await?;
            writer.flush().await?;
            writer.into_inner()
        };
        Ok(Self::from(bytes))
    }
}

pub async fn get_length_prefixed_reader<R>(stream: &mut R) -> io::Result<Take<&mut R>>
where
    R: io::AsyncRead + Unpin + Send,
{
    let length: u64 = match i32::from(McVarint::deserialize_read(stream).await?).try_into() {
        Ok(value) => value,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "failed to convert packet length from i32 to u64. It was probably negative",
            ))
        }
    };

    // This will only read a single packet
    Ok(stream.take(length))
}
