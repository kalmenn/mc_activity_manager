use std::process::{ExitStatus, Stdio};

use tokio::{
    process::{Command, ChildStdin, Child},
    io::{self, AsyncWriteExt}
};

pub struct McServer {
    process: Child,
    stdin: ChildStdin,
}

impl McServer {
    // pub fn from_command(command: &str) -> io::Result<McServer> {
    //     Self::with_args(command, &[])
    // }

    pub fn with_args(command: &str, args: &[&str]) -> io::Result<McServer> {
        println!("\n\x1b[38;2;0;200;0mStarting minecraft server as child process\x1b[0m\n");

        let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()?;

        let stdin = child.stdin
        .take()
        .expect("Could not bind to stdin of child process");

        Ok(McServer { process: child, stdin })
    }

    pub async fn stop(&mut self) -> io::Result<ExitStatus> {
        self.stdin.write_all("stop\n".as_bytes())
        .await
        .expect("Could not write to stdin of child process");

        self.wait_for_exit().await
    }

    pub async fn wait_for_exit(&mut self) -> io::Result<ExitStatus> {
        self.process.wait().await
    }
}