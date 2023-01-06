use crate::mc_protocol::data_types::McVarint;

pub enum HandshakePacket{
    Handshake {
        protocol_version: McVarint,
        server_address: String,
        server_port: u16,
        next_state: NextState,
    },
}

pub enum NextState {
    Status,
    Login
}