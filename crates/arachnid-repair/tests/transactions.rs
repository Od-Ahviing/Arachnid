use arachnid_core::{AppError, AppResult};
use arachnid_repair::{
    Action, Image, Journal, OperationPlan, Phase, Resources, Target, apply, pending, prepare,
    rollback,
};
use std::{collections::BTreeMap, fs, sync::atomic::AtomicBool};

#[derive(Default)]
struct MemoryResources {
    values: BTreeMap<Target, Image>,
    writes: usize,
    fail_at: Option<usize>,
    fail_after_at: Option<usize>,
}
impl Resources for MemoryResources {
    fn read(&self, target: &Target) -> AppResult<Option<Image>> {
        Ok(self.values.get(target).cloned())
    }
    fn write(&mut self, target: &Target, image: &Option<Image>) -> AppResult<()> {
        self.writes += 1;
        if self.fail_at == Some(self.writes) {
            return Err(AppError::new("injectedFailure"));
        }
        if let Some(image) = image {
            self.values.insert(target.clone(), image.clone());
        } else {
            self.values.remove(target);
        }
        if self.fail_after_at == Some(self.writes) {
            return Err(AppError::new("injectedFailureAfterWrite"));
        }
        Ok(())
    }
}
fn image(value: u8) -> Option<Image> {
    Some(Image {
        value_type: 4,
        bytes: vec![value, 0, 0, 0],
    })
}

#[test]
fn large_save_transaction_compensates_failure_after_the_old_32_action_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let mut plan = OperationPlan {
        schema_version: 1,
        installation: "synthetic".into(),
        actions: (0..256)
            .map(|i| Action {
                target: Target::File {
                    path: format!("photo-{i}.fpop"),
                },
                before: Some(Image {
                    value_type: 0,
                    bytes: vec![1],
                }),
                after: Some(Image {
                    value_type: 0,
                    bytes: vec![2],
                }),
            })
            .collect(),
    };
    let mut resources = baseline(&plan);
    let expected = resources.values.clone();
    let mut journal = prepare(plan.clone(), &resources, &path).unwrap();
    let digest = journal.digest.clone();
    resources.fail_after_at = Some(40);
    assert!(
        apply(
            &mut journal,
            &digest,
            &mut resources,
            &path,
            &AtomicBool::new(false),
            || Ok(())
        )
        .is_err()
    );
    assert_eq!(resources.values, expected);
    assert!(!pending(&path));
    plan.actions.push(Action {
        target: Target::File {
            path: "excess.fpop".into(),
        },
        before: None,
        after: Some(Image {
            value_type: 0,
            bytes: vec![1],
        }),
    });
    assert_eq!(plan.validate().unwrap_err().code, "invalidPlan");
}

#[test]
fn transaction_context_is_bounded_and_bound_to_the_approval_digest() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let resources = baseline(&plan);
    let mut journal = arachnid_repair::prepare_with_context(
        plan.clone(),
        &resources,
        &path,
        Some(serde_json::json!({"domain":"synthetic", "originals":"fixture"})),
    )
    .unwrap();
    assert_ne!(journal.digest, plan.digest().unwrap());
    assert_eq!(Journal::load(&path).unwrap().digest, journal.digest);
    journal.context.as_mut().unwrap()["originals"] = "tampered".into();
    journal.save(&path).unwrap();
    assert!(Journal::load(&path).is_err());
    let oversized = directory.path().join("oversized.json");
    assert!(
        arachnid_repair::prepare_with_context(
            plan,
            &resources,
            &oversized,
            Some(serde_json::json!({"data":"x".repeat(32769)}))
        )
        .is_err()
    );
    assert!(!oversized.exists());
}
fn plan() -> OperationPlan {
    OperationPlan {
        schema_version: 1,
        installation: "synthetic".into(),
        actions: vec![
            Action {
                target: Target::Registry {
                    section: "Display".into(),
                    name: "Width".into(),
                },
                before: image(1),
                after: image(2),
            },
            Action {
                target: Target::Registry {
                    section: "Display".into(),
                    name: "Height".into(),
                },
                before: None,
                after: image(3),
            },
        ],
    }
}
fn baseline(plan: &OperationPlan) -> MemoryResources {
    MemoryResources {
        values: plan
            .actions
            .iter()
            .filter_map(|a| a.before.clone().map(|v| (a.target.clone(), v)))
            .collect(),
        ..Default::default()
    }
}

#[test]
fn consent_staleness_cancellation_and_gate_never_mutate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let mut resources = baseline(&plan);
    let mut journal = prepare(plan.clone(), &resources, &path).unwrap();
    assert!(!pending(&path));
    assert!(
        apply(
            &mut journal,
            "wrong",
            &mut resources,
            &path,
            &AtomicBool::new(false),
            || Ok(())
        )
        .is_err()
    );
    let digest = journal.digest.clone();
    assert!(
        apply(
            &mut journal,
            &digest,
            &mut resources,
            &path,
            &AtomicBool::new(false),
            || Err(AppError::new("gameRunning"))
        )
        .is_err()
    );
    resources
        .values
        .insert(plan.actions[0].target.clone(), image(9).unwrap());
    assert_eq!(
        apply(
            &mut journal,
            &digest,
            &mut resources,
            &path,
            &AtomicBool::new(false),
            || Ok(())
        )
        .unwrap_err()
        .code,
        "stalePlan"
    );
    assert_eq!(resources.writes, 0);
    resources = baseline(&plan);
    assert_eq!(
        apply(
            &mut journal,
            &digest,
            &mut resources,
            &path,
            &AtomicBool::new(true),
            || Ok(())
        )
        .unwrap_err()
        .code,
        "cancelled"
    );
    assert_eq!(resources.writes, 0);
    assert!(!pending(&path));
}

#[test]
fn multi_action_failure_restores_exact_types_and_original_absence() {
    for fail_at in 1..=2 {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal.json");
        let plan = plan();
        let mut resources = baseline(&plan);
        let expected = resources.values.clone();
        resources.fail_at = Some(fail_at);
        let mut journal = prepare(plan, &resources, &path).unwrap();
        let digest = journal.digest.clone();
        assert!(
            apply(
                &mut journal,
                &digest,
                &mut resources,
                &path,
                &AtomicBool::new(false),
                || Ok(())
            )
            .is_err()
        );
        assert_eq!(journal.phase, Phase::RolledBack);
        assert_eq!(resources.values, expected);
        assert_eq!(Journal::load(&path).unwrap().phase, Phase::RolledBack);
    }
}

#[test]
fn committed_operation_can_be_restored_without_deleting_external_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let mut resources = baseline(&plan);
    let mut journal = prepare(plan.clone(), &resources, &path).unwrap();
    let digest = journal.digest.clone();
    apply(
        &mut journal,
        &digest,
        &mut resources,
        &path,
        &AtomicBool::new(false),
        || Ok(()),
    )
    .unwrap();
    assert_eq!(journal.phase, Phase::Committed);
    assert!(!pending(&path));
    resources
        .values
        .insert(plan.actions[1].target.clone(), image(99).unwrap());
    assert_eq!(
        rollback(&mut journal, &mut resources, &path)
            .unwrap_err()
            .code,
        "recoveryRequired"
    );
    assert_eq!(
        resources.values[&plan.actions[1].target],
        image(99).unwrap()
    );
    assert_eq!(resources.values[&plan.actions[0].target], image(1).unwrap());
    assert!(pending(&path));
}

#[test]
fn each_interrupted_intent_recovers_idempotently_before_or_after_write() {
    let plan = plan();
    for intent in 1..=plan.actions.len() {
        for after_write in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("journal.json");
            let mut resources = baseline(&plan);
            let expected = resources.values.clone();
            let mut journal = prepare(plan.clone(), &resources, &path).unwrap();
            for action in &plan.actions[..intent - 1] {
                resources.write(&action.target, &action.after).unwrap();
            }
            journal.intent_count = intent;
            journal.phase = Phase::Applying;
            journal.save(&path).unwrap();
            if after_write {
                let action = &plan.actions[intent - 1];
                resources.write(&action.target, &action.after).unwrap();
            }
            let mut recovered = Journal::load(&path).unwrap();
            rollback(&mut recovered, &mut resources, &path).unwrap();
            rollback(&mut recovered, &mut resources, &path).unwrap();
            assert_eq!(resources.values, expected);
            assert!(!pending(&path));
        }
    }
}

#[test]
fn damaged_journal_and_preimage_or_future_schema_block_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    fs::write(&path, b"{truncated").unwrap();
    assert!(pending(&path));
    fs::remove_file(&path).unwrap();
    let plan = plan();
    let resources = baseline(&plan);
    let mut journal = prepare(plan, &resources, &path).unwrap();
    journal.plan.actions[0].before = image(42);
    journal.save(&path).unwrap();
    assert!(pending(&path));
    journal.schema_version = 2;
    journal.save(&path).unwrap();
    assert!(Journal::load(&path).is_err());
}

#[test]
fn file_actions_restore_original_bytes_and_remove_only_owned_additions() {
    use arachnid_core::coordination::InstallationLease;
    use arachnid_repair::registry::InstallationResources;
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::write(root.path().join("existing.ini"), b"original\r\nunknown=7").unwrap();
    let lease = InstallationLease::acquire(root.path(), &data.path().join("locks")).unwrap();
    let mut resources = InstallationResources { lease: &lease };
    let target = Target::File {
        path: "existing.ini".into(),
    };
    let added = Target::File {
        path: "new.ini".into(),
    };
    let value = Some(Image {
        value_type: 0,
        bytes: b"changed".to_vec(),
    });
    let plan = OperationPlan {
        schema_version: 1,
        installation: lease.id(),
        actions: vec![
            Action {
                before: resources.read(&target).unwrap(),
                target: target.clone(),
                after: value.clone(),
            },
            Action {
                before: None,
                target: added,
                after: value,
            },
        ],
    };
    let path = data.path().join("journal.json");
    let mut journal = prepare(plan, &resources, &path).unwrap();
    let digest = journal.digest.clone();
    apply(
        &mut journal,
        &digest,
        &mut resources,
        &path,
        &AtomicBool::new(false),
        || Ok(()),
    )
    .unwrap();
    rollback(&mut journal, &mut resources, &path).unwrap();
    assert_eq!(
        fs::read(root.path().join("existing.ini")).unwrap(),
        b"original\r\nunknown=7"
    );
    assert!(!root.path().join("new.ini").exists());
}

#[test]
fn dangerous_windows_paths_and_duplicate_targets_are_rejected() {
    for path in [
        "../out", "/out", "x:stream", "a\\b", "CON.ini", "dir/../a", "foo.", "LPT1", "a//b",
    ] {
        assert!(
            arachnid_repair::filesystem::validate_relative(path).is_err(),
            "{path}"
        );
    }
    let mut plan = plan();
    plan.actions.push(plan.actions[0].clone());
    assert!(plan.validate().is_err());
}

#[test]
fn unapplied_preview_does_not_block_launch_but_incomplete_writes_do() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let resources = baseline(&plan);
    let mut journal = prepare(plan, &resources, &path).unwrap();
    assert!(!pending(&path));
    assert_eq!(Journal::load(&path).unwrap().intent_count, 0);
    for phase in [Phase::Applying, Phase::RollingBack, Phase::RecoveryRequired] {
        journal.phase = phase;
        journal.intent_count = 1;
        journal.save(&path).unwrap();
        assert!(pending(&path), "{phase:?}");
    }
    // A malformed preview cannot use the no-write exception.
    journal.phase = Phase::Prepared;
    journal.save(&path).unwrap();
    assert!(pending(&path));
    assert_eq!(resources.writes, 0);
}

#[test]
fn reported_failure_after_a_write_is_compensated_from_the_durable_intent() {
    for fail_after in 1..=2 {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal.json");
        let plan = plan();
        let mut resources = baseline(&plan);
        let expected = resources.values.clone();
        resources.fail_after_at = Some(fail_after);
        let mut journal = prepare(plan, &resources, &path).unwrap();
        let digest = journal.digest.clone();
        assert_eq!(
            apply(
                &mut journal,
                &digest,
                &mut resources,
                &path,
                &AtomicBool::new(false),
                || Ok(())
            )
            .unwrap_err()
            .code,
            "injectedFailureAfterWrite"
        );
        assert_eq!(resources.values, expected);
        assert_eq!(Journal::load(&path).unwrap().phase, Phase::RolledBack);
    }
}

#[test]
fn expired_or_future_dated_approval_never_writes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let mut resources = baseline(&plan);
    let mut journal = prepare(plan, &resources, &path).unwrap();
    let digest = journal.digest.clone();
    let now = arachnid_repair::now().unwrap();
    for timestamp in [now - 601, now + 60] {
        journal.prepared_at = timestamp;
        journal.save(&path).unwrap();
        assert_eq!(
            apply(
                &mut journal,
                &digest,
                &mut resources,
                &path,
                &AtomicBool::new(false),
                || Ok(())
            )
            .unwrap_err()
            .code,
            "approvalInvalid"
        );
        assert_eq!(resources.writes, 0);
        assert!(!pending(&path));
    }
}

#[test]
fn incorrect_readback_is_preserved_as_a_conflict_instead_of_reporting_success() {
    struct CorruptWriter(MemoryResources);
    impl Resources for CorruptWriter {
        fn read(&self, target: &Target) -> AppResult<Option<Image>> {
            self.0.read(target)
        }
        fn write(&mut self, target: &Target, _: &Option<Image>) -> AppResult<()> {
            self.0.write(target, &image(99))
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let mut resources = CorruptWriter(baseline(&plan));
    let mut journal = prepare(plan.clone(), &resources, &path).unwrap();
    let digest = journal.digest.clone();
    assert_eq!(
        apply(
            &mut journal,
            &digest,
            &mut resources,
            &path,
            &AtomicBool::new(false),
            || Ok(())
        )
        .unwrap_err()
        .code,
        "recoveryRequired"
    );
    assert_eq!(resources.0.writes, 1);
    assert_eq!(resources.read(&plan.actions[0].target).unwrap(), image(99));
    assert_eq!(Journal::load(&path).unwrap().phase, Phase::RecoveryRequired);
    assert!(pending(&path));
}

#[test]
fn failed_compensation_keeps_recoverable_intents_and_can_be_retried() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let mut resources = baseline(&plan);
    let expected = resources.values.clone();
    let mut journal = prepare(plan, &resources, &path).unwrap();
    let digest = journal.digest.clone();
    apply(
        &mut journal,
        &digest,
        &mut resources,
        &path,
        &AtomicBool::new(false),
        || Ok(()),
    )
    .unwrap();
    resources.fail_at = Some(3);
    assert!(rollback(&mut journal, &mut resources, &path).is_err());
    assert_eq!(Journal::load(&path).unwrap().phase, Phase::RecoveryRequired);
    assert!(pending(&path));
    resources.fail_at = None;
    let mut recovered = Journal::load(&path).unwrap();
    rollback(&mut recovered, &mut resources, &path).unwrap();
    assert_eq!(resources.values, expected);
    assert!(!pending(&path));
}

#[test]
fn game_start_between_actions_defers_compensation_until_the_game_is_stopped() {
    use arachnid_repair::rollback_guarded;
    use std::cell::Cell;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let mut resources = baseline(&plan);
    let expected = resources.values.clone();
    let mut journal = prepare(plan, &resources, &path).unwrap();
    let digest = journal.digest.clone();
    let checks = Cell::new(0);
    let result = apply(
        &mut journal,
        &digest,
        &mut resources,
        &path,
        &AtomicBool::new(false),
        || {
            checks.set(checks.get() + 1);
            if checks.get() >= 3 {
                Err(AppError::new("gameAlreadyRunning"))
            } else {
                Ok(())
            }
        },
    );
    assert_eq!(result.unwrap_err().code, "recoveryRequired");
    assert_eq!(resources.writes, 1);
    assert!(pending(&path));
    assert!(
        rollback_guarded(&mut journal, &mut resources, &path, || Err(AppError::new(
            "gameAlreadyRunning"
        )))
        .is_err()
    );
    assert_eq!(resources.writes, 1);
    rollback_guarded(&mut journal, &mut resources, &path, || Ok(())).unwrap();
    assert_eq!(resources.values, expected);
    assert!(!pending(&path));
}

#[test]
fn cancellation_between_actions_restores_the_first_action_without_writing_the_second() {
    use std::cell::Cell;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let mut resources = baseline(&plan);
    let expected = resources.values.clone();
    let mut journal = prepare(plan, &resources, &path).unwrap();
    let digest = journal.digest.clone();
    let checks = Cell::new(0);
    let cancelled = AtomicBool::new(false);
    assert_eq!(
        apply(
            &mut journal,
            &digest,
            &mut resources,
            &path,
            &cancelled,
            || {
                checks.set(checks.get() + 1);
                if checks.get() == 3 {
                    cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(())
            }
        )
        .unwrap_err()
        .code,
        "cancelled"
    );
    assert_eq!(resources.writes, 2);
    assert_eq!(resources.values, expected);
    assert!(!pending(&path));
}

#[cfg(windows)]
#[test]
fn journal_locked_after_first_write_keeps_disk_intent_for_a_later_recovery() {
    use std::{os::windows::fs::OpenOptionsExt, path::PathBuf};
    struct LockedJournal {
        resources: MemoryResources,
        path: PathBuf,
        lock: Option<fs::File>,
    }
    impl Resources for LockedJournal {
        fn read(&self, target: &Target) -> AppResult<Option<Image>> {
            self.resources.read(target)
        }
        fn write(&mut self, target: &Target, image: &Option<Image>) -> AppResult<()> {
            self.resources.write(target, image)?;
            if self.resources.writes == 1 {
                self.lock = Some(
                    fs::OpenOptions::new()
                        .read(true)
                        .share_mode(0)
                        .open(&self.path)
                        .unwrap(),
                );
            }
            Ok(())
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.json");
    let plan = plan();
    let expected = baseline(&plan).values;
    let mut resources = LockedJournal {
        resources: baseline(&plan),
        path: path.clone(),
        lock: None,
    };
    let mut journal = prepare(plan, &resources, &path).unwrap();
    let digest = journal.digest.clone();
    assert_eq!(
        apply(
            &mut journal,
            &digest,
            &mut resources,
            &path,
            &AtomicBool::new(false),
            || Ok(())
        )
        .unwrap_err()
        .code,
        "journalUnavailable"
    );
    assert_eq!(resources.resources.writes, 1);
    assert!(pending(&path));
    resources.lock = None;
    let mut recovered = Journal::load(&path).unwrap();
    assert_eq!(recovered.phase, Phase::Applying);
    assert_eq!(recovered.intent_count, 1);
    rollback(&mut recovered, &mut resources, &path).unwrap();
    assert_eq!(resources.resources.values, expected);
    assert!(!pending(&path));
}
