use std::process::Command;
fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    let git_hash = match output {
        Err(e) => {
            println!(
                "'subbers' is expected to be built from a git repository, using 'dev' for version: error: {e}"
            );
            println!("cargo:rustc-env=GIT_HASH=dev");
            return;
        }
        Ok(o) => String::from_utf8(o.stdout).expect("valid utf8 not found"),
    };

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // TODO: confirm required - tell cargo to monitor current head and typical main branch head.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/main")
}
