use std::process::Command;
use std::io::{self, Write};

fn main() -> Result<(), String> {
    let output = Command::new("sass")
        .arg("scss/:public/css/")
        .output()
        .expect("failed to execute sass");
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    if !output.status.success() {
        Err(format!("sass failed with {}", output.status))
    } else {
        Ok(())
    }
}
