mod spoofer;
mod minecraft_server_runner;

fn main() {
    loop {
        spoofer::wait_for_start_request();
        minecraft_server_runner::start_server();
    }
}