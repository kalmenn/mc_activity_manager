use std::{
    net::SocketAddr,
    borrow::BorrowMut
};

use tokio::{
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    io::{self, BufReader, BufWriter, AsyncReadExt, AsyncWriteExt}
};

use crate::mc_protocol::{
    data_types::{McVarint, LengthPrefixed},
    McProtocol,
    ProtocolVersion,
    Role,
    ConnectionState,
    Packet, 
    generic_packets::{
        self,
        GenericPacket,
        serverbound::{
            HandshakePacket,
            NextState,
            server_list_ping::{is_packet_server_list_ping, ServerListPingPacket}
        },
    },
};

pub struct Codec {
    reader: BufReader<OwnedReadHalf>,
    writer: BufWriter<OwnedWriteHalf>,
    connection_state: ConnectionState,
    role: Role,
    protocol_version: Option<ProtocolVersion>,
}

impl Codec {
    /// Handles a connection from a given TcpStream
    fn with_version_and_role(stream: TcpStream, protocol_version: Option<ProtocolVersion>, role: Role) -> Self {
        let (read_half, write_half) = stream.into_split();
        Codec { 
            reader: BufReader::new(read_half),
            writer: BufWriter::new(write_half),
            connection_state: ConnectionState::Handshaking,
            role,
            protocol_version,
        }
    }

    pub fn new_server(stream: TcpStream) -> Self {
        Self::with_version_and_role(stream, None, Role::Server)
    }

    pub async fn new_client(server_addr: SocketAddr) -> io::Result<Self> {
        let stream = TcpStream::connect(server_addr).await?;
        Ok(Self::with_version_and_role(stream, None, Role::Server))
    }

    pub async fn read_packet(&mut self) -> io::Result<Packet> {
        if let ConnectionState::Handshaking = self.connection_state {
            if is_packet_server_list_ping::<io::Result<bool>>(self.reader.get_mut()).await? {
                return Ok(
                    Packet::Generic(
                        GenericPacket::Serverbound(
                            generic_packets::serverbound::ServerboundPacket::ServerListPing(
                                ServerListPingPacket::deserialize_read(&mut self.reader).await?
                            )
                        )
                    )
                )
            }
        };

        let packet_length: u64 = match i32::from(McVarint::deserialize_read(&mut self.reader).await?).try_into() {
            Ok(value) => value,
            Err(_) => return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "failed to convert packet length from i32 to u64. It was probably negative"
            )),
        };

        // This will only read a single packet
        let mut packet_reader = self.reader.borrow_mut().take(packet_length);

        let packet = if let ConnectionState::Handshaking = self.connection_state {
            let packet = HandshakePacket::deserialize_read(&mut packet_reader).await?;

            self.protocol_version = Some(i32::from(packet.protocol_version.clone()).try_into()?);

            self.connection_state = match packet.next_state {
                NextState::Status => ConnectionState::Status,
                NextState::Login => ConnectionState::Login,
            };

            Packet::Generic(GenericPacket::Serverbound(generic_packets::serverbound::ServerboundPacket::Handshake(packet)))
        } else {
            Packet::deserialize_read(
                &mut packet_reader,
                &self.connection_state,
                &self.role,
                &self.protocol_version
            ).await?
        };

        let remaining_bytes = packet_reader.limit();
        if remaining_bytes != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("{remaining_bytes} bytes were not consumed by the implementation of deserialize_read")
            ))
        }

        Ok(packet)
    }

    pub async fn send_packet(&mut self, packet: impl McProtocol) -> io::Result<()> {
        LengthPrefixed::from({
            let mut writer = BufWriter::new(Vec::<u8>::new());
            packet.serialize_write(&mut writer).await?;
            writer.flush().await?;
            writer.into_inner()
        }).serialize_write(&mut self.writer).await?;

        self.writer.flush().await
    }
}