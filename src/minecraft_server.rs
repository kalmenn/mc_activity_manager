use std::process::{Command, ExitStatus};

pub fn start_server() {
    println!("Exit status: {:?}\n", run_as_child_process());
}

fn run_as_child_process() -> std::io::Result<ExitStatus> {
    println!("\x1b[38;2;0;200;0m\nStarting minecraft server in child process\x1b[0m\n");
    
    // netcat as proof of concept
    // The child process can indeed bind to the same port
    let mut child = Command::new("nc")
    .args(vec!("-l","-p 6969"))
    .spawn()?;

    child.wait()
}