use std::process::{Command, ExitStatus};

pub fn run_server() -> std::io::Result<ExitStatus> {

    // netcat as proof of concept
    // The child process can indeed bind to the same port
    let mut child = Command::new("nc")
    .args(vec!("-l","-p 6969"))
    .spawn()?;

    child.wait()
}