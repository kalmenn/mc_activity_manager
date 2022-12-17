use std::process::{Command, ExitStatus};

pub fn start_server() {
    println!("Exit status: {:?}", run_as_child_process());
}

fn run_as_child_process() -> std::io::Result<ExitStatus> {
    println!("\n\x1b[38;2;0;200;0mStarting minecraft server in child process\x1b[0m\n");

    // TODO: allow for a custom command to be specified in a config file

    // netcat as proof of concept
    // The child process can indeed bind to the same port
    // let mut child = Command::new("nc")
    // .args(vec!("-l","-p 6969"))
    // .spawn()?;

    // sl is faster to test with
    let mut child = Command::new("sl").spawn()?;

    child.wait()
}