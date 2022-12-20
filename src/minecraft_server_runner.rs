use std::process::ExitStatus;

use tokio::{
    process::Command,
    io
};

pub async fn run_server() {
    let status = run_as_child_process().await;
    println!("Process exited on status: {:?}", status);
}

async fn run_as_child_process() -> io::Result<ExitStatus> {
    println!("\n\x1b[38;2;0;200;0mStarting minecraft server in child process\x1b[0m\n");

    // TODO: allow for a custom command to be specified in a config file

    // netcat as proof of concept
    // The child process can indeed bind to the same port
    // let mut child = Command::new("nc")
    // .stdin(Stdio::piped())
    // .stdout(Stdio::piped())
    // .args(vec!("-l", "-p 6969"))
    // .spawn()?;

    // sl is faster to test with
    // let mut child = Command::new("sl").spawn()?;

    let mut child = Command::new("/bin/bash")
    .arg("./start.sh")
    .spawn()?;

    child.wait().await
}