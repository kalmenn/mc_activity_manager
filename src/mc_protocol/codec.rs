use tokio::{
    net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}},
    io::{BufReader, BufWriter, self}
};

use crate::mc_protocol::McProtocol;

use super::packets::{
    serverbound,
    ServerboundPacket,
    ClientboundPacket,
};

enum ConnectionState {
    Handshaking,
    Status,
    Login,
}

pub struct Codec {
    reader: BufReader<OwnedReadHalf>,
    writer: BufWriter<OwnedWriteHalf>,
    connection_state: ConnectionState,
}

impl Codec {
    /// Handles a connection from a given TcpStream
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        let (read_half, write_half) = stream.into_split();
        Ok(Codec { 
            reader: BufReader::new(read_half),
            writer: BufWriter::new(write_half),
            connection_state: ConnectionState::Handshaking
        })
    }

    pub async fn read_packet(&mut self) -> io::Result<ServerboundPacket> {
        match self.connection_state {
            ConnectionState::Handshaking => {
                let packet = serverbound::HandshakePacket::deserialize_read(&mut self.reader).await?;
                match packet.next_state {
                    serverbound::NextState::Login => self.connection_state = ConnectionState::Login,
                    serverbound::NextState::Status => self.connection_state = ConnectionState::Status,
                };
                Ok(ServerboundPacket::Handshake(packet))
            },
            _ => todo!(),
        }
    }

    pub async fn send_packet(&mut self, packet: impl ClientboundPacket) -> io::Result<()> {
        packet.serialize_write(&mut self.writer).await
    }
}