mod spoofer;
mod varint;
mod codec;
mod minecraft_server;

fn main() {
    loop {
        spoofer::listen();
        minecraft_server::start_server();
    }
}