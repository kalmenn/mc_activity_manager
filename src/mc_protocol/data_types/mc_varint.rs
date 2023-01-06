use tokio::io::{
    self,
    AsyncRead,
    AsyncReadExt,
    AsyncWriteExt,
};

#[derive(Clone)]
pub struct McVarint(Vec<u8>);

#[async_trait::async_trait]
impl crate::mc_protocol::McProtocol for McVarint {
    async fn serialize_write<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::AsyncWrite + Unpin + Send
    {
        writer.write_all(&self.0).await?;
        Ok(())
    }

    async fn deserialize_read<R>(reader: &mut R) -> io::Result<Self> 
    where
        Self: std::marker::Sized,
        R: io::AsyncRead + Unpin + Send
    {
        let mut bytes = Vec::<u8>::new();

        loop {
            let byte = reader.read_u8().await?;
            bytes.push(byte);
            if bytes.len() > 5 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "varints can't be over 5 bytes long"))
            }
            if byte < 128 {break}
        }

        Ok(McVarint(bytes))
    }
}

impl From<i32> for McVarint {
    fn from(value: i32) -> Self {
        if value == 0 {
            return McVarint(vec![0])
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
        for i in 0..bytes.len()-1 {
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

use bitvec::prelude::*;

pub fn into_varint<I>(number: I) -> Vec<u8> 
where I: BitStore {
    let bits = number.view_bits::<Lsb0>();

    let mut bytes: Vec<u8> = vec![0; (bits.len() + 6) / 7];

    // Fill bytes with groups of 7 bits form the bits BitSlice
    for bit in bits.iter().by_vals().enumerate() {
        bytes[(bit.0) / 7] += 2_u8.pow((bit.0 % 7) as u32) * match bit.1 {
            true => 1,
            false => 0
        };
    }

    // Remove trailing null bytes
    bytes = bytes.into_iter()
    .enumerate()
    .take_while(|byte| byte.1 > 0 || byte.0 == 0)
    .map(|byte| byte.1)
    .collect();

    // Add continuation bits
    for i in 0..bytes.len()-1 {
        bytes[i] += 128;
    }

    bytes
}

/// Reads a varint from an async reader and consumes its bytes.
pub async fn from_reader<R>(reader: &mut R) -> Result<u32, io::Error> 
where
    R: AsyncRead + std::marker::Unpin
{
    let value = {
        let mut varint_reader = VarintReader::new();
        loop {
            if let Some(byte) = varint_reader.try_byte(
                reader.read_u8().await?
            )?
            {
                break byte
            }
        }
    };

    Ok(value)
}

/// Used to read a varint byte by byte.
///
/// Push bytes with `VarintReader.try_byte()` and check the return value.
/// Keep pushing until `Ok(Some)`, has been returned.
///
/// # Example
///
/// ```no_run
/// let mut bytes = vec![0b11110100_u8, 0b00000011_u8].into_iter();
///
/// let mut reader = VarintReader::new();
///
/// let value = loop {
///     if let Some(value) = varint_reader.try_byte(bytes.next().unwrap()).unwrap() {
///         break value
///     }
/// };
///
/// assert_eq!(500, value);
/// ```
pub struct VarintReader {
    /// The number of bytes that have been read
    length: usize,
    /// The data that has been read
    data: u32,
    /// Have all the bytes of the varint been read?
    /// If yes, we shouldn't push any more bytes.
    complete: bool
}

impl VarintReader {
    /// Creates a new reader
    pub fn new() -> VarintReader {
        VarintReader{length: 0, data: 0, complete: false}
    }

    /// Add a byte of data to the reader.
    ///
    /// Returns a `Ok(Some)` containing the value encoded by the varint
    /// when all the bytes have been read, `Ok(None)` if more are needed.
    ///
    /// Returns and `Err(io::Erorr)` if the varint is over 4 bytes long,
    /// as the specification doesn't allow for such.
    pub fn try_byte(&mut self, byte: u8) -> io::Result<Option<u32>> {
        if !self.complete {
            // prepend the 7 bits to the current data
            self.data += ((byte & 0b01111111) as u32) << (7 * self.length);
            
            // Check for continuation bit
            if byte >= 128 {
                self.length += 1;
                if self.length > 4 {
                    return Err(io::Error::from(io::ErrorKind::InvalidData));
                }
                Ok(None)
            } else {
                self.complete = true;
                Ok(Some(self.data))
            }
        } else {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mc_protocol::McProtocol;

    use super::*;

    #[test]
    fn into_varint_test() {
        let varint = into_varint(968_usize);
        let expect: Vec<u8> = vec!(0b11001000, 0b00000111);
        assert_eq!(
            expect,
            varint
        );
    }

    #[test]
    fn varint_reader_test() {
        let mut reader = VarintReader::new();
        
        let mut bytes = vec![0b11110100_u8, 0b00000011_u8, 0, 0, 0].into_iter();

        loop {
            if let Some(value) = reader.try_byte(bytes.next().unwrap()).unwrap() {
                return assert_eq!(500, value);
            }
        }
    }

    #[test]
    fn varint_chain() {
        let mut bytes = into_varint(6969_u32).into_iter();

        let mut reader = VarintReader::new();

        let value = loop {
            if let Some(value) = reader.try_byte(bytes.next().unwrap()).unwrap() {
                break value
            }
        };

        assert_eq!(6969, value);
    }

    #[tokio::test]
    async fn read_varint_test() {
        let bytes = [0b11110100_u8, 0b00000011_u8, 0, 0, 0];
        let mut reader = io::BufReader::new(&bytes[..]);
        assert_eq!(
            500,
            from_reader(&mut reader).await.unwrap()
        );
    }

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