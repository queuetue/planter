use crate::model::Phase;
use std::process::Command;

pub async fn execute(phase: &Phase) -> Result<(), String> {
    let desc = &phase.spec.description;
    println!("(Simulating Python execution for '{}')", desc);

    // Replace this with real logic — for now we simulate success
    // Use a safer command that doesn't involve shell escaping issues
    let output = Command::new("python3")
        .arg("-c")
        .arg("print('Executing phase')")
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "Script failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("{}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

#[cfg(test)]
mod tests;
