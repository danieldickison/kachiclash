use std::process::Command;

fn main() -> Result<(), String> {
    println!("running sass");
    let status = Command::new("sass")
        .arg("scss/:public/css/")
        .status()
        .expect("run sass");
    if !status.success() {
        return Err(format!("sass failed with {}", status));
    }

    println!("running npx tsc");
    let status = Command::new("npx").arg("tsc").status().expect("run tsc");
    if !status.success() {
        return Err(format!("tsc failed with {}", status));
    }

    Ok(())
}
