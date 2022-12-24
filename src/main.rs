mod spoofer;
mod minecraft_server_runner;
mod mc_protocol;

use spoofer::Spoofer;
use minecraft_server_runner::McServer;

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
        {
            let mut server = McServer::with_args(
                "/bin/bash", 
                &[
                    "start.sh"
                ]
            ).unwrap();

            let exit_status = server.wait_for_exit().await.unwrap();
            println!("Server exited on status: {}", exit_status);
        }
    }
}