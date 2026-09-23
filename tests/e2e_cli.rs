// End-to-end test for the CLI binary. `CARGO_BIN_EXE_*` is set by cargo for
// integration tests, so the binary is built automatically before running.
use std::process::Command;

#[test]
fn cli_lists_build_tables() {
    let output = Command::new(env!("CARGO_BIN_EXE_corporate-nayose"))
        .arg("--help")
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("build-tables"),
        "--help should list the build-tables subcommand"
    );
}
