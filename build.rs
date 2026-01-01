use std::process::Command;
fn main() {
    // https://stackoverflow.com/questions/43753491/include-git-commit-hash-as-string-into-rust-program/44407625#44407625
    let output = Command::new("git").args(&["describe", "--tags"]).output().unwrap();
    let git_description = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_DESCRIPTION={}", git_description);
}
