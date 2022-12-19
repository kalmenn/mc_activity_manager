mod spoofer;
mod minecraft_server_runner;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    loop {
        spoofer::wait_for_start_request().await;
        minecraft_server_runner::start_server();
    }
}