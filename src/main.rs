mod spoofer;
mod minecraft_server_runner;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    loop {
        spoofer::Spoofer::new().wait_for_start_request().await;
        minecraft_server_runner::run_server().await;
    }
}