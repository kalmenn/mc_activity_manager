mod spoofer;
mod minecraft_server_runner;

use spoofer::Spoofer;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    loop {
        {
            let mut spoofer = Spoofer::new()
            .set_debug(true)
            .set_port(6969);
    
            spoofer.start_listening();
    
            spoofer.wait_for_start_request().await;
        }

        minecraft_server_runner::run_server().await;
    }
}