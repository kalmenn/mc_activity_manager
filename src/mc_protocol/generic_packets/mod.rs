use self::serverbound::ServerboundPacket;
pub mod serverbound;

#[derive(Debug)]
pub enum GenericPacket {
    Serverbound(ServerboundPacket),
}