pub mod handshake;
pub use handshake::{HandshakePacket, NextState};

pub mod server_list_ping;
pub use server_list_ping::ServerListPingPacket;

#[derive(Debug)]
pub enum ServerboundPacket {
    Handshake(HandshakePacket),
    ServerListPing(ServerListPingPacket),
}