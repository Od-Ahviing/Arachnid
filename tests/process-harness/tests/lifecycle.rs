use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

const FIXTURE: &str = env!("CARGO_BIN_EXE_arachnid-process-harness");

struct Scenario {
    directory: TempDir,
    child: Child,
}

impl Scenario {
    fn start(mode: &str, code: u8, payload: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let working_directory = directory.path().join("répertoire avec espaces");
        fs::create_dir(&working_directory).unwrap();
        let executable =
            working_directory.join(format!("process fixture{}", std::env::consts::EXE_SUFFIX));
        fs::copy(FIXTURE, &executable).unwrap();
        let child = Command::new(executable)
            .args([mode, "20", &code.to_string()])
            .arg(directory.path())
            .arg(payload)
            .current_dir(working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Self { directory, child }
    }

    fn report(&self, name: &str) -> Value {
        let path = self.directory.path().join(name);
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Ok(bytes) = fs::read(&path)
                && let Ok(report) = serde_json::from_slice(&bytes)
            {
                return report;
            }
            assert!(Instant::now() < deadline, "missing complete report: {name}");
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn release(&self) {
        fs::write(self.directory.path().join("release"), b"release").unwrap();
    }

    fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "fixture did not exit");
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Scenario {
    fn drop(&mut self) {
        let _ = fs::write(self.directory.path().join("release"), b"release");
        // Cleanup targets only the original fixture handle, never a game process or an arbitrary PID.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn worker_stays_running_until_released_and_returns_exact_exit_code() {
    for code in [0, 23] {
        let mut scenario = Scenario::start("worker", code, "test");
        let started = scenario.report("started.json");
        assert_eq!(started["pid"], scenario.child.id());
        assert!(scenario.child.try_wait().unwrap().is_none());
        scenario.release();
        assert_eq!(scenario.wait().code(), Some(i32::from(code)));
        assert_eq!(scenario.report("finished.json")["exitCode"], code);
        assert_eq!(
            scenario.child.try_wait().unwrap().unwrap().code(),
            Some(i32::from(code))
        );
    }
}

#[test]
fn executable_and_working_directory_support_unicode_and_spaces_with_literal_arguments() {
    let payload = "écho & | > marker.txt ; $(ignored) \"quoted\" %PATH%";
    let mut scenario = Scenario::start("worker", 0, payload);
    let started = scenario.report("started.json");
    assert_eq!(started["payload"], payload);
    let actual = PathBuf::from(started["workingDirectory"].as_str().unwrap());
    let expected = scenario.directory.path().join("répertoire avec espaces");
    assert_eq!(
        fs::canonicalize(actual).unwrap(),
        fs::canonicalize(&expected).unwrap()
    );
    assert!(!expected.join("marker.txt").exists());
    scenario.release();
    assert!(scenario.wait().success());
}

#[test]
fn relay_exit_does_not_mean_child_exit_and_parent_closure_does_not_kill_child() {
    let mut scenario = Scenario::start("relay", 23, "detached child");
    let relay = scenario.report("relay.json");
    let started = scenario.report("started.json");
    assert_eq!(relay["childPid"], started["pid"]);
    assert_ne!(started["pid"], scenario.child.id());
    assert!(scenario.wait().success());
    assert!(!scenario.directory.path().join("finished.json").exists());
    scenario.release();
    assert_eq!(scenario.report("finished.json")["exitCode"], 23);
}

#[test]
fn invalid_fixture_arguments_fail_without_creating_reports() {
    let directory = tempfile::tempdir().unwrap();
    for args in [
        vec![
            "worker",
            "1001",
            "0",
            directory.path().to_str().unwrap(),
            "test",
        ],
        vec![
            "worker",
            "0",
            "256",
            directory.path().to_str().unwrap(),
            "test",
        ],
        vec![
            "unknown",
            "0",
            "0",
            directory.path().to_str().unwrap(),
            "test",
        ],
        vec!["worker", "0", "0", "relative-path", "test"],
    ] {
        let status = Command::new(FIXTURE)
            .args(args)
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(2));
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn missing_executable_is_a_spawn_error() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join(Path::new("missing.exe"));
    let error = Command::new(missing).spawn().unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}
