use std::borrow::Cow;
use std::process::Command;

const MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
const MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
const PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");

fn main() {
    let git_hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|x| String::from_utf8(x.stdout).ok())
        .or_else(|| option_env!("").map(std::string::ToString::to_string));

    let ver_string = format!("{MAJOR}.{MINOR}.{PATCH}");

    let version: Cow<_> = match &git_hash {
        Some(git_hash) => format!("{ver_string} ({})", &git_hash[..8]).into(),
        None => ver_string.into(),
    };

    println!("cargo:rustc-env=VERSION_AND_GIT_HASH={version}");
}
