// https://thepacketgeek.com/rust/tcpstream/lines-codec/

use std::io::{self, Write, Read};
use std::net::TcpStream;
use crate::varints::into_varint;

/// Handles reading and writing of packets. 
pub struct Codec {
    reader: io::BufReader<TcpStream>,
    writer: io::BufWriter<TcpStream>,
}

impl Codec {
    /// Encapsulate a TcpStream with buffered reader/writer functionality
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        let writer = io::BufWriter::new(stream.try_clone()?);
        let reader = io::BufReader::new(stream);
        Ok(Self { reader, writer })
    }

    /// Write the given message (while prepending packet length) to the TcpStream
    pub fn send_message(&mut self, message: Vec<u8>) -> io::Result<()> {
        let mut packet = into_varint(message.len());
        packet.reserve(message.len());
        for byte in message.iter() {
            packet.push(*byte);
        }
        self.writer.write(&packet)?;
        self.writer.flush()
    }

    fn read_byte(&mut self) -> io::Result<u8> {
        let mut byte = [0_u8; 1];
        self.reader.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    /// Read a received message from the TcpStream
    pub fn read_message(&mut self) -> io::Result<Vec<u8>> {

        // The packet is prefixed with a varint encoding its size
        let mut packet_length = PacketLength::unknown();

        // Reading the varint
        while let PacketLength::Reading(varint) = &mut packet_length {
            let data = self.read_byte()?;
            varint.data += ((data & 0b01111111) as u32) << (7 * varint.length);
            if data >= 128 {
                // Found continuation byte
                varint.length += 1;
                if varint.length > 4 {
                    return Err(io::Error::from(io::ErrorKind::InvalidData));
                }
            } else {
                // End of varint
                packet_length.to_known();
            }
        }

        let mut packet_body = vec![0u8; packet_length.length()? as usize];
        self.reader.read_exact(&mut packet_body)?;
        Ok(packet_body)
    }
}

struct VarintBuffer {
    length: usize,
    data: u32
}

/// Used as a buffer while reading the VarInt that encodes the packet length
enum PacketLength {
    Known(u32),
    Reading(VarintBuffer)
}

impl PacketLength {
    /// Initialises with an empty VarInt
    fn unknown() -> PacketLength {
        PacketLength::Reading(VarintBuffer{length: 0, data: 0})
    }

    /// Returns the packet length or an error if it isn't known
    fn length(&self) -> io::Result<u32> {
        match self {
            PacketLength::Known(packet_length) => Ok(*packet_length),
            PacketLength::Reading(_) => Err(io::Error::from(io::ErrorKind::InvalidData))
        }
    }

    /// Freezes the packet length as a known value
    fn to_known(&mut self) {
        if let Self::Reading(varint) = self {
            *self = Self::Known(varint.data);
        }
    }
}