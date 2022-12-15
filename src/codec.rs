// https://thepacketgeek.com/rust/tcpstream/lines-codec/

use std::io::{self, BufRead, Write};
use std::net::TcpStream;

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

    /// Write the given message (appending a newline) to the TcpStream
    pub fn send_message(&mut self, message: Vec<u8>) -> io::Result<()> {
        self.writer.write(&message)?;
        self.writer.flush();
        Ok(())
    }

    /// Read a received message from the TcpStream
    pub fn read_message(&mut self) -> io::Result<Vec<u8>> {
        todo!();
        // let mut line = String::new();
        // self.reader.read_line(&mut line)?;
        // line.pop(); // Remove the trailing "\n"
        // Ok(line)
    }
}