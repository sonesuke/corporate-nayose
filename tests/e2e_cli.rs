// End-to-end tests for the CLI binary. `CARGO_BIN_EXE_*` is set by cargo for
// integration tests, so the binary is built automatically before running.
use std::process::Command;

#[test]
fn cli_prints_greeting() {
    let output = Command::new(env!("CARGO_BIN_EXE_corporate-nayose"))
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, world!\n");
}
