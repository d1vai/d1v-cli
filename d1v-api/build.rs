use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let output = Command::new(rustc)
        .arg("--version")
        .output()
        .expect("failed to run rustc");

    // "rustc 1.94.0 (4a4ef493e 2026-03-02)" -> "1.94.0"
    let output = String::from_utf8(output.stdout).unwrap();
    let version = output.split_whitespace().nth(1).unwrap_or("unknown");

    println!("cargo:rustc-env=D1V_RUSTC_VERSION={version}");
}
