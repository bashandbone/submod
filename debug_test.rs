use std::process::Command;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = std::env::temp_dir().join("debug_git_test");
    fs::create_dir_all(&temp_dir)?;
    let remote_dir = temp_dir.join("test.git");
    
    println!("Creating remote at: {}", remote_dir.display());
    
    // Initialize bare repository
    let output = Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&remote_dir)
        .output()?;
    
    println!("Init output: {}", String::from_utf8_lossy(&output.stderr));
    
    // Create a working copy
    let work_copy = temp_dir.join("test_work");
    let output = Command::new("git")
        .args(["clone", remote_dir.to_str().unwrap(), work_copy.to_str().unwrap()])
        .output()?;
    
    println!("Clone output: {}", String::from_utf8_lossy(&output.stderr));
    
    // Check branch
    let output = Command::new("git")
        .args(["branch"])
        .current_dir(&work_copy)
        .output()?;
    
    println!("Branch output: {}", String::from_utf8_lossy(&output.stdout));
    
    // Try to create main branch
    let output = Command::new("git")
        .args(["checkout", "-b", "main"])
        .current_dir(&work_copy)
        .output()?;
    
    println!("Checkout output: {}", String::from_utf8_lossy(&output.stderr));
    
    // Add content
    fs::write(work_copy.join("test.txt"), "test content")?;
    
    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&work_copy)
        .output()?;
    
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&work_copy)
        .output()?;
    
    // Add and commit
    let output = Command::new("git")
        .args(["add", "."])
        .current_dir(&work_copy)
        .output()?;
    
    println!("Add output: {}", String::from_utf8_lossy(&output.stderr));
    
    let output = Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&work_copy)
        .output()?;
    
    println!("Commit output: {}", String::from_utf8_lossy(&output.stderr));
    
    // Push
    let output = Command::new("git")
        .args(["push", "origin", "main"])
        .current_dir(&work_copy)
        .output()?;
    
    println!("Push output: {}", String::from_utf8_lossy(&output.stderr));
    
    // Test cloning the remote
    let test_clone = temp_dir.join("test_clone");
    let output = Command::new("git")
        .args(["clone", remote_dir.to_str().unwrap(), test_clone.to_str().unwrap()])
        .output()?;
    
    println!("Test clone output: {}", String::from_utf8_lossy(&output.stderr));
    
    // Check if files exist
    println!("Files in test clone:");
    for entry in fs::read_dir(&test_clone)? {
        let entry = entry?;
        println!("  {}", entry.file_name().to_string_lossy());
    }
    
    Ok(())
}
