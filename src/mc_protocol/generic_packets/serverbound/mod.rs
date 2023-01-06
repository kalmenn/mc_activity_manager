pub mod handshake;
pub use handshake::{HandshakePacket, NextState};

#[derive(Debug)]
pub enum ServerboundPacket {
    Handshake(HandshakePacket),
}