use serde::Serialize;
use std::{
    env, fs, io,
    path::PathBuf,
    process::{self, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Started {
    pid: u32,
    working_directory: PathBuf,
    payload: String,
}

fn write_report(directory: &std::path::Path, name: &str, report: impl Serialize) -> io::Result<()> {
    fs::write(directory.join(name), serde_json::to_vec(&report)?)
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 5
        || !matches!(
            args[0].as_str(),
            "worker"
                | "relay"
                | "lease"
                | "transaction"
                | "save-writer"
                | "save-crash"
                | "game-window"
        )
    {
        return Err("expected: worker|relay delay-ms exit-code report-directory payload".into());
    }
    let delay: u64 = args[1].parse()?;
    let exit_code: u8 = args[2].parse()?;
    let directory = PathBuf::from(&args[3]);
    if delay > 1_000 || !directory.is_absolute() || !directory.is_dir() {
        return Err(
            "delay must be <=1000 ms and report directory must exist and be absolute".into(),
        );
    }

    if args[0] == "transaction" {
        return crash_transaction(&directory, delay as usize, exit_code, &args[4]);
    }
    if args[0] == "game-window" {
        return game_window(&directory, delay);
    }

    if args[0] == "save-writer" {
        let source = directory.join("saves");
        if !source.is_dir() {
            return Err("missing fixture saves directory".into());
        }
        fs::write(source.join("slot.dat"), b"fragment")?;
        fs::write(source.join("progress.dat"), b"fragment")?;
        fs::write(directory.join("writer-ready"), b"ready")?;
        thread::sleep(Duration::from_millis(delay));
        fs::write(source.join("slot.tmp"), b"complete-slot")?;
        fs::rename(source.join("slot.tmp"), source.join("slot.dat"))?;
        fs::write(source.join("progress.dat"), b"complete-progress")?;
        return Ok(0);
    }

    if args[0] == "save-crash" {
        use arachnid_saveguard::{SnapshotStage, SourceProfile, Store};
        let source = directory.join("saves");
        let backup = directory.join("backups");
        let profile = SourceProfile::new(&source, &backup).map_err(|e| e.code)?;
        let store = Store::open(&backup, profile).map_err(|e| e.code)?;
        let target = match args[4].as_str() {
            "files" => SnapshotStage::FilesCopied,
            "manifest" => SnapshotStage::ManifestSynced,
            "published" => SnapshotStage::Published,
            _ => return Err("invalid save crash checkpoint".into()),
        };
        store
            .snapshot_with_checkpoint("manual", Duration::ZERO, |stage| {
                if stage == target {
                    process::exit(i32::from(exit_code));
                }
            })
            .map_err(|e| e.code)?;
        return Err("save checkpoint was not reached".into());
    }

    if args[0] == "relay" {
        let child = Command::new(env::current_exe()?)
            .arg("worker")
            .args(&args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        write_report(
            &directory,
            "relay.json",
            serde_json::json!({ "childPid": child.id() }),
        )?;
        // This fixture deliberately exits before its child; production supervision must not infer game exit from this relay.
        return Ok(0);
    }

    let _lease = if args[0] == "lease" {
        Some(
            arachnid_core::coordination::InstallationLease::acquire(
                &directory,
                std::path::Path::new(&args[4]),
            )
            .map_err(|error| error.code)?,
        )
    } else {
        None
    };
    write_report(
        &directory,
        "started.json",
        Started {
            pid: process::id(),
            working_directory: env::current_dir()?,
            payload: args[4].clone(),
        },
    )?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !directory.join("release").is_file() {
        if Instant::now() >= deadline {
            return Err("fixture release deadline exceeded".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(delay));
    write_report(
        &directory,
        "finished.json",
        serde_json::json!({ "exitCode": exit_code }),
    )?;
    Ok(i32::from(exit_code))
}

fn crash_transaction(
    directory: &std::path::Path,
    crash_action: usize,
    code: u8,
    point: &str,
) -> Result<i32, Box<dyn std::error::Error>> {
    use arachnid_core::{AppResult, coordination::InstallationLease};
    use arachnid_repair::{
        Action, Image, OperationPlan, Resources, Target, registry::InstallationResources,
    };
    use std::sync::atomic::AtomicBool;
    if !(1..=2).contains(&crash_action)
        || !matches!(
            point,
            "before" | "after" | "rollback-before" | "rollback-after" | "prepared" | "committed"
        )
    {
        return Err("invalid crash point".into());
    }
    struct CrashWriter<'a> {
        resources: InstallationResources<'a>,
        writes: usize,
        crash_action: usize,
        code: u8,
        after: bool,
        armed: bool,
    }
    impl Resources for CrashWriter<'_> {
        fn read(&self, target: &Target) -> AppResult<Option<Image>> {
            self.resources.read(target)
        }
        fn write(&mut self, target: &Target, image: &Option<Image>) -> AppResult<()> {
            self.writes += 1;
            if self.armed && self.writes == self.crash_action && !self.after {
                process::exit(i32::from(self.code));
            }
            self.resources.write(target, image)?;
            if self.armed && self.writes == self.crash_action && self.after {
                process::exit(i32::from(self.code));
            }
            Ok(())
        }
    }
    let lease = InstallationLease::acquire(&directory.join("game"), &directory.join("locks"))
        .map_err(|error| error.code)?;
    let mut resources = CrashWriter {
        resources: InstallationResources { lease: &lease },
        writes: 0,
        crash_action,
        code,
        after: point.ends_with("after"),
        armed: matches!(point, "before" | "after"),
    };
    let first = Target::File {
        path: "original.ini".into(),
    };
    let plan = OperationPlan {
        schema_version: 1,
        installation: lease.id(),
        actions: vec![
            Action {
                before: resources.read(&first).map_err(|error| error.code)?,
                target: first,
                after: Some(Image {
                    value_type: 0,
                    bytes: b"changed".to_vec(),
                }),
            },
            Action {
                before: None,
                target: Target::File {
                    path: "added.ini".into(),
                },
                after: Some(Image {
                    value_type: 0,
                    bytes: b"added".to_vec(),
                }),
            },
        ],
    };
    let path = directory.join("journal.json");
    let mut journal =
        arachnid_repair::prepare(plan, &resources, &path).map_err(|error| error.code)?;
    if point == "prepared" {
        process::exit(i32::from(code));
    }
    let digest = journal.digest.clone();
    arachnid_repair::apply(
        &mut journal,
        &digest,
        &mut resources,
        &path,
        &AtomicBool::new(false),
        || Ok(()),
    )
    .map_err(|error| error.code)?;
    if point == "committed" {
        process::exit(i32::from(code));
    }
    if point.starts_with("rollback-") {
        resources.armed = true;
        resources.writes = 0;
        arachnid_repair::rollback(&mut journal, &mut resources, &path)
            .map_err(|error| error.code)?;
    }
    Err("expected an injected process exit".into())
}

#[cfg(windows)]
fn game_window(directory: &std::path::Path, delay: u64) -> Result<i32, Box<dyn std::error::Error>> {
    use windows_sys::Win32::{System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*};
    fs::write(directory.join("window-waiting"), b"waiting")?;
    thread::sleep(Duration::from_millis(delay));
    unsafe {
        let class: Vec<u16> = "ArachnidOriginalGameFixture"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let instance = GetModuleHandleW(std::ptr::null());
        let definition = WNDCLASSW {
            lpfnWndProc: Some(DefWindowProcW),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..Default::default()
        };
        if RegisterClassW(&definition) == 0 {
            return Err("fixture class registration failed".into());
        }
        let window = CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            100,
            100,
            640,
            400,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if window.is_null() {
            return Err("fixture window creation failed".into());
        }
        ShowWindow(window, SW_SHOW);
        fs::write(directory.join("window-visible"), b"visible")?;
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut message = MSG::default();
        while !directory.join("release-window").exists() && Instant::now() < deadline {
            while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            thread::sleep(Duration::from_millis(10));
        }
        DestroyWindow(window);
        Ok(0)
    }
}

#[cfg(not(windows))]
fn game_window(_: &std::path::Path, _: u64) -> Result<i32, Box<dyn std::error::Error>> {
    Err("Windows only".into())
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("process fixture: {error}");
            process::exit(2);
        }
    }
}
