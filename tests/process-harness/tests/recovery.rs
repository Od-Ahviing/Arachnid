use arachnid_core::coordination::InstallationLease;
use arachnid_repair::{Journal, Phase, registry::InstallationResources};
use std::{fs, process::Command};

#[test]
fn abrupt_original_process_exit_recovers_each_durable_file_intent_on_windows() {
    for action in ["1", "2"] {
        for point in ["before", "after", "rollback-before", "rollback-after"] {
            let directory = tempfile::tempdir().unwrap();
            let game = directory.path().join("game");
            fs::create_dir(&game).unwrap();
            fs::write(game.join("original.ini"), b"baseline\r\nunknown=1").unwrap();
            let status = Command::new(env!("CARGO_BIN_EXE_arachnid-process-harness"))
                .args(["transaction", action, "99"])
                .arg(directory.path())
                .arg(point)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(99));
            let path = directory.path().join("journal.json");
            let mut journal = Journal::load(&path).unwrap();
            if point.starts_with("rollback-") {
                assert_eq!(journal.phase, Phase::RollingBack);
                assert_eq!(journal.intent_count, 2);
            } else {
                assert_eq!(journal.phase, Phase::Applying);
                assert_eq!(journal.intent_count.to_string(), action);
            }
            let lease = InstallationLease::acquire(&game, &directory.path().join("locks")).unwrap();
            let mut resources = InstallationResources { lease: &lease };
            arachnid_repair::rollback(&mut journal, &mut resources, &path).unwrap();
            assert_eq!(
                fs::read(game.join("original.ini")).unwrap(),
                b"baseline\r\nunknown=1"
            );
            assert!(!game.join("added.ini").exists());
            assert_eq!(Journal::load(&path).unwrap().phase, Phase::RolledBack);
        }
    }
}

#[test]
fn abrupt_exit_at_preview_or_commit_preserves_the_correct_launch_decision() {
    for (point, expected_phase, expected_bytes) in [
        ("prepared", Phase::Prepared, b"baseline".as_slice()),
        ("committed", Phase::Committed, b"changed".as_slice()),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path().join("game");
        fs::create_dir(&game).unwrap();
        fs::write(game.join("original.ini"), b"baseline").unwrap();
        let status = Command::new(env!("CARGO_BIN_EXE_arachnid-process-harness"))
            .args(["transaction", "1", "99"])
            .arg(directory.path())
            .arg(point)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(99));
        let path = directory.path().join("journal.json");
        let mut journal = Journal::load(&path).unwrap();
        assert_eq!(journal.phase, expected_phase);
        assert_eq!(fs::read(game.join("original.ini")).unwrap(), expected_bytes);
        assert!(!arachnid_repair::pending(&path));
        let lease = InstallationLease::acquire(&game, &directory.path().join("locks")).unwrap();
        arachnid_repair::rollback(
            &mut journal,
            &mut InstallationResources { lease: &lease },
            &path,
        )
        .unwrap();
        assert_eq!(fs::read(game.join("original.ini")).unwrap(), b"baseline");
        assert!(!game.join("added.ini").exists());
    }
}
