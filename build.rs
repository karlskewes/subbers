use std::process::Command;
fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("'subbers' is expected to be built from a git repository");
    let git_hash = String::from_utf8(output.stdout).expect("valid utf8 not found");
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // TODO: confirm required - tell cargo to monitor current head and typical main branch head.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/main")
}
