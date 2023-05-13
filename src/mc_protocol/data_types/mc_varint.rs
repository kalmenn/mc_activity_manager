use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use crate::mc_protocol::McProtocol;

#[derive(Clone)]
pub struct McVarint(Vec<u8>);

#[async_trait::async_trait]
impl McProtocol for McVarint {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send,
    {
        writer.write_all(&self.0).await?;
        Ok(())
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self>
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send,
    {
        let mut bytes = Vec::<u8>::new();

        loop {
            let byte = reader.read_u8().await?;
            bytes.push(byte);
            if bytes.len() > 5 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "varints can't be over 5 bytes long",
                ));
            }
            if byte < 128 {
                break;
            }
        }

        Ok(McVarint(bytes))
    }
}

impl From<i32> for McVarint {
    fn from(value: i32) -> Self {
        if value == 0 {
            return McVarint(vec![0]);
        }

        // Create a vector of the right size
        let number_of_bits = 32 - value.leading_zeros() as usize;
        let number_of_bytes: usize = (number_of_bits / 7) + (number_of_bits % 7 > 0) as usize;
        let mut bytes = vec![0_u8; number_of_bytes];

        // Make groups of 7 bits
        for bit in 0..number_of_bits {
            bytes[(bit) / 7] += 2_u8.pow((bit % 7) as u32) * (value >> bit & 1) as u8;
        }

        // Add continuation bits
        for i in 0..bytes.len() - 1 {
            bytes[i] += 128;
        }

        McVarint(bytes)
    }
}

impl From<McVarint> for i32 {
    fn from(value: McVarint) -> Self {
        let mut number: i32 = 0;
        for byte in value.0.iter().enumerate() {
            number += ((byte.1 & 0b01111111) as i32) << (7 * byte.0)
        }
        number
    }
}

impl std::fmt::Debug for McVarint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", i32::from(self.clone()))
    }
}

impl TryFrom<McVarint> for u32 {
    type Error = io::Error;

    fn try_from(value: McVarint) -> Result<Self, Self::Error> {
        let value_i32: i32 = value.into();
        Ok(match value_i32.try_into() {
            Ok(value) => value,
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("value {value_i32} outside of u32 bounds"),
                ))
            }
        })
    }
}

impl TryFrom<u32> for McVarint {
    type Error = io::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let value_i32: i32 = match value.try_into() {
            Ok(value) => value,
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("value {value} outside of i32 bounds"),
                ))
            }
        };
        Ok(McVarint::from(value_i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mc_protocol::McProtocol;
    use tokio::io::{BufReader, BufWriter};

    #[tokio::test]
    async fn mc_varint_create_test() {
        let varint = McVarint::from(500);

        let bytes = {
            let mut writer = BufWriter::new(Vec::<u8>::new());
            varint.serialize_write(&mut writer).await.unwrap();
            writer.flush().await.unwrap();
            writer.into_inner()
        };

        println!("{bytes:?}");
        assert_eq!(vec![0b11110100_u8, 0b00000011_u8], bytes);
    }

    #[tokio::test]
    async fn mc_varint_read_test() {
        let bytes = vec![0b11110100_u8, 0b00000011_u8];
        let mut reader = BufReader::new(bytes.as_slice());

        let number = i32::from(McVarint::deserialize_read(&mut reader).await.unwrap());

        assert_eq!(500, number);
    }

    #[tokio::test]
    async fn mc_varint_null() {
        let varint = McVarint::from(0);

        let bytes = {
            let mut writer = BufWriter::new(Vec::<u8>::new());
            varint.serialize_write(&mut writer).await.unwrap();
            writer.flush().await.unwrap();
            writer.into_inner()
        };

        assert_eq!(vec![0b00000000_u8], bytes);
    }
}
