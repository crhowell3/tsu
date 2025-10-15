use std::process::Command;

const MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
const MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
const PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

fn main() {
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let ver_string = format!("{MAJOR}.{MINOR}.{PATCH}");

    let version = format!("{} ({})", ver_string, git_hash);

    println!("cargo:rustc-env=VERSION_AND_GIT_HASH={}", version);
}
