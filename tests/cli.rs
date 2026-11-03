use std::process::{Command, Output};

fn run(argument: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plex-tray"))
        .arg(argument)
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent/plex-tray-test-session",
        )
        .env(
            "DBUS_SYSTEM_BUS_ADDRESS",
            "unix:path=/nonexistent/plex-tray-test-system",
        )
        .output()
        .expect("run plex-tray")
}

#[test]
fn help_works_without_a_desktop_or_service() {
    let output = run("--help");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for option in ["--no-start", "--monochrome", "--color", "--version"] {
        assert!(text.contains(option), "missing {option} from help");
    }
}

#[test]
fn version_works_without_dbus() {
    let output = run("--version");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        concat!("plex-tray ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn invalid_options_fail_before_connecting_to_dbus() {
    let output = run("--invalid-option");
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Unknown argument: --invalid-option")
    );
}
