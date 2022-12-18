// Derived from: https://thepacketgeek.com/rust/tcpstream/lines-codec/

use std::io::{self, Write, Read};
use std::net::TcpStream;
use super::varint::{into_varint, VarintReader};

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
        self.writer.write_all(&packet)?;
        self.writer.flush()
    }

    fn read_byte(&mut self) -> io::Result<u8> {
        let mut byte = [0_u8; 1];
        self.reader.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    /// Read a received message from the TcpStream
    pub fn read_message(&mut self) -> io::Result<Vec<u8>> {
        // Read the Varint encoding the size of the packet
        let mut varint_reader = VarintReader::new();
        let packet_length = loop {
            if let Some(value) = varint_reader.try_byte(self.read_byte()?)? {
                break value
            }
        };

        // Read the packet body
        let mut packet_body = vec![0u8; packet_length as usize];
        self.reader.read_exact(&mut packet_body)?;
        Ok(packet_body)
    }
}