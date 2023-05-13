mod handshake;
pub use handshake::{HandshakePacket, NextState};

mod server_list_ping;
pub use server_list_ping::{is_packet_server_list_ping, ServerListPingPacket};

#[derive(Debug)]
pub enum Generic {
    Handshake(HandshakePacket),
    ServerListPing(ServerListPingPacket),
}
