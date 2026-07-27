use std::process::Command;

// Records the commit being built, for the version saved in results files
fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok());

    if let Some(commit) = commit {
        println!("cargo:rustc-env=CUTERGAMES_COMMIT={}", commit.trim());
    }
}
