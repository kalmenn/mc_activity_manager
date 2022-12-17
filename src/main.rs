mod spoofer;
mod minecraft_server_runner;

fn main() {
    loop {
        spoofer::listen();
        minecraft_server_runner::start_server();
    }
}