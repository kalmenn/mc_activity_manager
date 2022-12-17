mod spoofer;
mod minecraft_server;

fn main() {
    loop {
        spoofer::listen();
        minecraft_server::start_server();
    }
}