use tokio::{
    io::{self, AsyncWriteExt, BufReader, BufWriter},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

use crate::mc_protocol::{
    data_types::{get_length_prefixed_reader, LengthPrefixed},
    serverbound_packets::{
        self,
        generic_packets::{is_packet_server_list_ping, NextState},
        Serverbound,
    },
    ConnectionState, McProtocol, ProtocolVersion, ProtocolVersionLevelDeserialize,
};

pub struct ServerCodec {
    reader: BufReader<OwnedReadHalf>,
    writer: BufWriter<OwnedWriteHalf>,
    connection_state: ConnectionState,
    protocol_version: Option<ProtocolVersion>,
}

impl ServerCodec {
    pub fn new(stream: TcpStream) -> Self {
        let (read_half, write_half) = stream.into_split();
        ServerCodec {
            reader: BufReader::new(read_half),
            writer: BufWriter::new(write_half),
            connection_state: ConnectionState::Handshaking,
            protocol_version: None,
        }
    }

    pub async fn read_packet(&mut self) -> io::Result<Serverbound> {
        if let ConnectionState::Handshaking = self.connection_state {
            if is_packet_server_list_ping::<io::Result<bool>>(self.reader.get_mut()).await? {
                return Ok(
                    Serverbound::Generic(
                        serverbound_packets::generic_packets::Generic::ServerListPing(
                            serverbound_packets::generic_packets::ServerListPingPacket::deserialize_read(&mut self.reader).await?
        )));
            }
        };

        // This will only read a single packet
        let mut packet_reader = get_length_prefixed_reader(&mut self.reader).await?;

        let packet = if let ConnectionState::Handshaking = self.connection_state {
            let packet = serverbound_packets::generic_packets::HandshakePacket::deserialize_read(
                &mut packet_reader,
            )
            .await?;

            self.protocol_version = Some(i32::from(packet.protocol_version.clone()).try_into()?);

            self.connection_state = match packet.next_state {
                NextState::Status => ConnectionState::Status,
                NextState::Login => ConnectionState::Login,
            };

            Serverbound::Generic(serverbound_packets::generic_packets::Generic::Handshake(
                packet,
            ))
        } else {
            Serverbound::deserialize_read(
                &mut packet_reader,
                self.connection_state,
                self.protocol_version
                    .expect("protocol version should be known by this point"),
            )
            .await?
        };

        let remaining_bytes = packet_reader.limit();
        if remaining_bytes != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("{remaining_bytes} bytes were not consumed by the implementation of deserialize_read")
            ));
        }

        Ok(packet)
    }

    pub async fn send_packet(&mut self, packet: impl McProtocol) -> io::Result<()> {
        LengthPrefixed::from_mc_protocol(packet)
            .await?
            .serialize_write(&mut self.writer)
            .await?;

        self.writer.flush().await
    }
}
