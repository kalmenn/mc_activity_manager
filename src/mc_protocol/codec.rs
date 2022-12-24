// Derived from: https://thepacketgeek.com/rust/tcpstream/lines-codec/

use tokio::{
    net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}},
    io::{BufReader, BufWriter, AsyncReadExt, AsyncWriteExt, self}
};
use super::data_types::varint::{into_varint, VarintReader};

/// Handles reading and writing of packets. 
pub struct Codec {
    reader: BufReader<OwnedReadHalf>,
    writer: BufWriter<OwnedWriteHalf>
}

impl Codec {
    /// Encapsulate a TcpStream with buffered reader/writer functionality
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        let (read, write) = stream.into_split();
        let reader = BufReader::new(read);
        let writer = BufWriter::new(write);
        Ok(Self { reader, writer })
    }

    /// Write the given message (while prepending packet length) to the TcpStream
    pub async fn send_message(&mut self, message: Vec<u8>) -> io::Result<()> {
        let mut packet = into_varint(message.len());
        packet.reserve(message.len());
        for byte in message.iter() {
            packet.push(*byte);
        }
        self.writer.write_all(&packet).await?;
        self.writer.flush().await
    }

    async fn read_byte(&mut self) -> io::Result<u8> {
        let mut byte = [0_u8; 1];
        self.reader.read_exact(&mut byte).await?;
        Ok(byte[0])
    }

    /// Read a received message from the TcpStream
    pub async fn read_message(&mut self) -> io::Result<Vec<u8>> {
        // Read the Varint encoding the size of the packet
        let mut varint_reader = VarintReader::new();
        let packet_length = loop {
            if let Some(value) = varint_reader.try_byte(self.read_byte().await?)? {
                break value
            }
        };

        // Read the packet body
        let mut packet_body = vec![0u8; packet_length as usize];
        self.reader.read_exact(&mut packet_body).await?;
        Ok(packet_body)
    }
}