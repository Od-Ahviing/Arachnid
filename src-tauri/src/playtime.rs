use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    version: u32,
    milliseconds: u64,
}

pub struct Playtime {
    path: PathBuf,
    record: Option<Record>,
    active: Option<Instant>,
    saved: Instant,
    dirty: bool,
}

impl Playtime {
    pub fn load(directory: &Path) -> Self {
        let path = directory.join("playtime.json");
        let record = match fs::File::open(&path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(1025).read_to_end(&mut bytes).ok().and_then(|_| {
                    (bytes.len() <= 1024)
                        .then(|| serde_json::from_slice::<Record>(&bytes).ok())
                        .flatten()
                        .filter(|record| record.version == 1)
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(Record {
                version: 1,
                milliseconds: 0,
            }),
            Err(_) => None,
        };
        Self {
            path,
            record,
            active: None,
            saved: Instant::now(),
            dirty: false,
        }
    }

    pub fn started(&mut self) {
        if self.active.is_none() {
            self.active = Some(Instant::now());
        }
    }

    pub fn observe(&mut self, running: bool) -> Option<u64> {
        self.observe_at(running, Instant::now())
    }

    fn observe_at(&mut self, running: bool, now: Instant) -> Option<u64> {
        let record = self.record.as_mut()?;
        if let Some(previous) = self.active {
            let elapsed = now.saturating_duration_since(previous).as_millis();
            record.milliseconds = record
                .milliseconds
                .saturating_add(elapsed.min(u64::MAX as u128) as u64);
            self.active = running.then_some(now);
            self.dirty |= elapsed > 0;
        }
        let seconds = record.milliseconds / 1000;
        if self.dirty && (!running || now.duration_since(self.saved) >= Duration::from_secs(30)) {
            if self.persist().is_ok() {
                self.dirty = false;
                self.saved = now;
            } else {
                return None;
            }
        }
        Some(seconds)
    }

    fn persist(&self) -> std::io::Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(self.path.parent().unwrap())?;
        serde_json::to_writer(&mut file, self.record.as_ref().unwrap())?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(&self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_started_sessions_and_persists_without_double_counting() {
        let dir = tempfile::tempdir().unwrap();
        let mut clock = Playtime::load(dir.path());
        assert_eq!(clock.observe(true), Some(0));
        clock.started();
        let start = clock.active.unwrap();
        assert_eq!(
            clock.observe_at(true, start + Duration::from_secs(35)),
            Some(35)
        );
        assert_eq!(Playtime::load(dir.path()).observe(false), Some(35));
        assert_eq!(
            clock.observe_at(false, start + Duration::from_secs(50)),
            Some(50)
        );
        assert_eq!(
            clock.observe_at(false, start + Duration::from_secs(90)),
            Some(50)
        );
        let mut reopened = Playtime::load(dir.path());
        reopened.started();
        let next = reopened.active.unwrap();
        assert_eq!(
            reopened.observe_at(false, next + Duration::from_secs(12)),
            Some(62)
        );
        assert_eq!(Playtime::load(dir.path()).observe(false), Some(62));
    }

    #[test]
    fn corrupt_future_and_oversized_records_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        for bytes in [
            b"broken".to_vec(),
            br#"{"version":2,"milliseconds":100}"#.to_vec(),
            vec![b' '; 1025],
        ] {
            fs::write(dir.path().join("playtime.json"), &bytes).unwrap();
            let mut clock = Playtime::load(dir.path());
            clock.started();
            assert_eq!(clock.observe(false), None);
            assert_eq!(fs::read(&clock.path).unwrap(), bytes);
        }
    }

    #[test]
    fn failed_checkpoint_retries_without_losing_time() {
        let dir = tempfile::tempdir().unwrap();
        let mut clock = Playtime::load(dir.path());
        clock.started();
        let start = clock.active.unwrap();
        fs::create_dir(&clock.path).unwrap();
        assert_eq!(
            clock.observe_at(false, start + Duration::from_secs(40)),
            None
        );
        fs::remove_dir(&clock.path).unwrap();
        assert_eq!(clock.observe(false), Some(40));
        assert_eq!(Playtime::load(dir.path()).observe(false), Some(40));
    }

    #[test]
    fn restart_after_interruption_keeps_checkpoint_without_counting_downtime() {
        let dir = tempfile::tempdir().unwrap();
        let mut clock = Playtime::load(dir.path());
        clock.started();
        let start = clock.active.unwrap();
        clock.started();
        assert_eq!(clock.active, Some(start));
        clock.observe_at(true, start + Duration::from_secs(35));
        clock.observe_at(true, start + Duration::from_secs(40));
        drop(clock);
        let mut reopened = Playtime::load(dir.path());
        assert_eq!(reopened.observe(false), Some(35));
        assert_eq!(reopened.observe(true), Some(35));
    }

    #[test]
    fn concurrent_status_reads_do_not_duplicate_elapsed_time() {
        let dir = tempfile::tempdir().unwrap();
        let mut clock = Playtime::load(dir.path());
        clock.started();
        let now = clock.active.unwrap() + Duration::from_secs(60);
        let clock = std::sync::Mutex::new(clock);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let clock = &clock;
                scope.spawn(move || {
                    assert_eq!(clock.lock().unwrap().observe_at(true, now), Some(60));
                });
            }
        });
        assert_eq!(Playtime::load(dir.path()).observe(false), Some(60));
    }
}
