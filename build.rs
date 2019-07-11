use std::process::Command;

fn main() -> Result<(), String> {
    let status = Command::new("sass")
        .arg("scss/:public/css/")
        .status()
        .expect("run sass");
    if !status.success() {
        Err(format!("sass failed with {}", status))
    } else {
        Ok(())
    }
}
