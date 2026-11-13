#![cfg(windows)]
use arachnid_core::coordination::InstallationLease;
use arachnid_launch::{ChannelKind, QualifiedChannel, Session};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
    thread,
    time::{Duration, Instant},
};

const FIXTURE: &str = env!("CARGO_BIN_EXE_arachnid-process-harness");
struct Scenario {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
}
impl Scenario {
    fn new() -> Self {
        let parent = tempfile::Builder::new()
            .prefix("Arachnid é space ")
            .tempdir()
            .unwrap();
        fs::copy(FIXTURE, parent.path().join("fixture.exe")).unwrap();
        fs::copy(FIXTURE, parent.path().join("Game.exe")).unwrap();
        Self {
            root: parent,
            data: tempfile::tempdir().unwrap(),
        }
    }
    fn channel(&self, relay: bool) -> QualifiedChannel {
        let image = arachnid_detector::inspect_pe(
            &self.root.path().join("fixture.exe"),
            "fixture.exe",
            &AtomicBool::new(false),
        )
        .unwrap();
        QualifiedChannel {
            executable: "fixture.exe".into(),
            sha256: image.sha256,
            arguments: vec![
                if relay { "relay" } else { "worker" }.into(),
                "10".into(),
                "23".into(),
                self.root.path().to_str().unwrap().into(),
                "literal & ; $(test)".into(),
            ],
            kind: if relay {
                ChannelKind::Relay
            } else {
                ChannelKind::Direct
            },
        }
    }
    fn lease(&self) -> InstallationLease {
        InstallationLease::acquire(self.root.path(), self.data.path()).unwrap()
    }
    fn wait_file(&self, name: &str) {
        let deadline = Instant::now() + Duration::from_secs(12);
        while !self.root.path().join(name).exists() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn release(&self) {
        fs::write(self.root.path().join("release"), b"done").unwrap();
    }
}
impl Drop for Scenario {
    fn drop(&mut self) {
        let _ = fs::write(self.root.path().join("release"), b"cleanup");
    }
}

#[test]
fn supervisor_observes_handle_exit_and_holds_exclusion_until_drop() {
    let scenario = Scenario::new();
    let channel = scenario.channel(false);
    let mut session = Session::start(scenario.lease(), &channel).unwrap();
    scenario.wait_file("started.json");
    assert_eq!(session.status().unwrap().state, "running");
    assert!(InstallationLease::acquire(scenario.root.path(), scenario.data.path()).is_err());
    scenario.release();
    let deadline = Instant::now() + Duration::from_secs(12);
    while session.status().unwrap().state == "running" {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(session.status().unwrap().exit_code, Some(23));
    assert_eq!(session.status().unwrap().state, "exited");
    drop(session);
    assert!(InstallationLease::acquire(scenario.root.path(), scenario.data.path()).is_ok());
}

#[test]
fn relay_is_unknown_after_parent_exit_and_closing_session_does_not_kill_child() {
    let scenario = Scenario::new();
    let channel = scenario.channel(true);
    let mut session = Session::start(scenario.lease(), &channel).unwrap();
    scenario.wait_file("started.json");
    let deadline = Instant::now() + Duration::from_secs(12);
    while session.status().unwrap().state == "running" {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(session.status().unwrap().state, "unknown");
    assert!(!scenario.root.path().join("finished.json").exists());
    drop(session);
    scenario.release();
    scenario.wait_file("finished.json");
}

#[test]
fn changed_binary_and_unbounded_channel_paths_are_rejected() {
    let scenario = Scenario::new();
    let mut channel = scenario.channel(false);
    channel.sha256 = "wrong".into();
    assert!(Session::start(scenario.lease(), &channel).is_err());
    assert!(!scenario.root.path().join("started.json").exists());
    channel.executable = "../fixture.exe".into();
    assert!(Session::start(scenario.lease(), &channel).is_err());
    channel.executable = "cmd.exe /c test".into();
    assert!(Session::start(scenario.lease(), &channel).is_err());
}

#[test]
fn independent_processes_cannot_hold_the_same_installation_lease() {
    let scenario = Scenario::new();
    let spawn = || {
        let mut command = Command::new(Path::new(FIXTURE));
        command
            .args(["lease", "0", "0"])
            .arg(scenario.root.path())
            .arg(scenario.data.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    };
    let mut first = spawn().spawn().unwrap();
    scenario.wait_file("started.json");
    assert_eq!(spawn().status().unwrap().code(), Some(2));
    scenario.release();
    assert!(first.wait().unwrap().success());
    assert!(spawn().status().unwrap().success());
}

#[test]
fn external_original_fixture_at_game_path_blocks_launch_until_it_exits() {
    let scenario = Scenario::new();
    let external_reports = scenario.root.path().join("external");
    fs::create_dir(&external_reports).unwrap();
    let mut external = Command::new(scenario.root.path().join("Game.exe"))
        .args(["worker", "0", "0"])
        .arg(&external_reports)
        .arg("external fixture")
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(12);
    while !external_reports.join("started.json").exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    assert!(arachnid_launch::ensure_game_stopped(scenario.root.path()).is_err());
    assert_eq!(
        Session::start(scenario.lease(), &scenario.channel(false))
            .err()
            .unwrap()
            .code,
        "gameAlreadyRunning"
    );
    fs::write(external_reports.join("release"), b"done").unwrap();
    assert!(external.wait().unwrap().success());
    arachnid_launch::ensure_game_stopped(scenario.root.path()).unwrap();
}
