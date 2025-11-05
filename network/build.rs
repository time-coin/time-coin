use std::process::Command;

fn main() {
    // Tell Cargo to rerun this build script if git HEAD changes
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads/main");

    // Get git commit hash
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    let git_hash = if let Ok(output) = output {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
