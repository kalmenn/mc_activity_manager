// Derived from: https://thepacketgeek.com/rust/tcpstream/lines-codec/

use tokio::{
    net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}},
    io::{BufReader, BufWriter, AsyncReadExt, AsyncWriteExt, self}
};
use super::data_types::mc_varint::{from_reader, into_varint};

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

    /// Read a received message from the TcpStream
    pub async fn read_message(&mut self) -> io::Result<Vec<u8>> {
        let packet_length = from_reader(&mut self.reader).await?;

        // Read the packet body
        let mut packet_body = vec![0u8; packet_length as usize];
        self.reader.read_exact(&mut packet_body).await?;
        Ok(packet_body)
    }
}