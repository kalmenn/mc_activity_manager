use tokio::{
    net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}},
    io::{self, BufReader, BufWriter, AsyncReadExt, AsyncWriteExt}
};

use crate::mc_protocol::{
    data_types::McVarint,
    McProtocol,
    ProtocolVersion,
};

use super::packets_761::{
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
    reader: Option<BufReader<OwnedReadHalf>>,
    writer: BufWriter<OwnedWriteHalf>,
    connection_state: ConnectionState,
    protocol_version: Option<ProtocolVersion>,
}

impl Codec {
    /// Handles a connection from a given TcpStream
    pub fn with_version(stream: TcpStream, protocol_version: Option<ProtocolVersion>) -> io::Result<Self> {
        let (read_half, write_half) = stream.into_split();
        Ok(Codec { 
            reader: Some(BufReader::new(read_half)),
            writer: BufWriter::new(write_half),
            connection_state: ConnectionState::Handshaking,
            protocol_version: protocol_version,
        })
    }

    pub fn new(stream: TcpStream) -> io::Result<Self> {
        Self::with_version(stream, None)
    }

    pub async fn read_packet(&mut self) -> io::Result<ServerboundPacket> {
        let packet_length: u64 = match i32::from(McVarint::deserialize_read(
            self.reader.as_mut().expect("reader should have been put back from the previous take()")
        ).await?).try_into() {
            Ok(value) => value,
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "failed to convert packet length from i32 to u64. It was probably negative"
            )),
        };

        dbg!(&packet_length);

        // This will only read a single packet
        let mut packet_reader = self.reader
            .take()
            .expect("reader should have been put back from the previous take()")
            .take(packet_length);

        let packet = match self.connection_state {
            ConnectionState::Handshaking => {
                let packet = serverbound::HandshakePacket::deserialize_read(&mut packet_reader).await?;
                match packet.next_state {
                    serverbound::NextState::Login => self.connection_state = ConnectionState::Login,
                    serverbound::NextState::Status => self.connection_state = ConnectionState::Status,
                };
                ServerboundPacket::Handshake(packet)
            },
            ConnectionState::Status => {
                let packet = serverbound::StatusPacket::deserialize_read(&mut packet_reader).await?;
                ServerboundPacket::Status(packet)
            },
            ConnectionState::Login => {
                let packet = serverbound::LoginPacket::deserialize_read(&mut packet_reader).await?;
                ServerboundPacket::Login(packet)
            },
        };

        dbg!(&packet);

        if let ServerboundPacket::Handshake(packet) = &packet {
            self.protocol_version = Some(i32::from(packet.protocol_version.clone()).try_into()?);
        }

        {
            let remaining_bytes = packet_reader.limit();
            if remaining_bytes != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("{remaining_bytes} bytes were not consumed by the implementation of deserialize_read")
                ))
            }
        }

        // We put back the reader of the full stream
        self.reader = Some(packet_reader.into_inner());
        Ok(packet)
    }

    pub async fn send_packet(&mut self, packet: impl ClientboundPacket) -> io::Result<()> {
        let packet_body = {
            let mut writer = BufWriter::new(Vec::<u8>::new());
            packet.serialize_write(&mut writer).await?;
            writer.flush().await?;
            writer.into_inner()
        };

        McVarint::from(
            match i32::try_from(packet_body.len()) {
                Ok(value) => value,
                Err(_) => return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "packet was longer than allowed"
                )),
            }
        ).serialize_write(&mut self.writer).await?;

        self.writer.write_all(packet_body.as_slice()).await?;

        self.writer.flush().await
    }
}