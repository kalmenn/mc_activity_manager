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

    return bytes;
}

use std::io;

/// Used to read a varint byte by byte.
///
/// Push bytes with `VarintReader.try_byte()` and check the return value.
/// Keep pushing until `Ok(Some)`, has been returned.
///
/// # Example
///
/// ```rust
/// let mut bytes = vec![0b11110100_u8, 0b00000011_u8, 0, 0, 0].into_iter();
/// 
/// let mut reader = VarintReader::new();
///
/// let value = loop {
///     if let Some(value) = varint_reader.try_byte(bytes.next().unwrap())? {
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
    
    /// Returns the value of the varint and locks it as known
    fn read_and_lock(&mut self) -> u32 {
        self.complete = true;
        self.data
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
                Ok(Some(self.read_and_lock()))
            }
        } else {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }
    
}

#[cfg(test)]
mod tests {
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
}