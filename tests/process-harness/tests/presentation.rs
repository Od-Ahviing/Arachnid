#![cfg(windows)]
use arachnid_launch::presentation::has_visible_game_window;
use std::{
    fs,
    os::windows::process::CommandExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn a_live_process_does_not_count_as_an_open_game_until_its_own_window_is_visible() {
    let fixture = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_arachnid-process-harness"))
        .args(["game-window", "1000", "0"])
        .arg(fixture.path())
        .arg("fixture")
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !fixture.path().join("window-waiting").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!has_visible_game_window(child.id()));
    while !fixture.path().join("window-visible").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(has_visible_game_window(child.id()));
    assert!(!has_visible_game_window(u32::MAX));
    fs::write(fixture.path().join("release-window"), b"exit").unwrap();
    assert!(child.wait().unwrap().success());
    assert!(!has_visible_game_window(child.id()));
}
