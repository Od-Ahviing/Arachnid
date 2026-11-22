use arachnid_saveguard::{SourceProfile, Store, source, statistics};
use std::{path::PathBuf, time::Duration};

fn main() -> Result<(), String> {
    let mut arguments = std::env::args_os().skip(1);
    let source = PathBuf::from(
        arguments
            .next()
            .ok_or("Usage: inspect_saves SOURCE_DIRECTORY")?,
    );
    if arguments.next().is_some() {
        return Err("Usage: inspect_saves SOURCE_DIRECTORY".into());
    }
    let before = source::stable(&source, Duration::from_millis(750)).map_err(|error| error.code)?;
    println!(
        "Files: {}, bytes: {}",
        before.len(),
        before
            .values()
            .map(|image| image.bytes.len())
            .sum::<usize>()
    );
    println!(
        "Native play time (seconds): {:?}",
        statistics::read_playtime(&source)
    );
    // Only the temporary destination is written; the selected source is never restored or modified.
    let destination = tempfile::tempdir().map_err(|error| error.to_string())?;
    let profile = SourceProfile::new(&source, destination.path()).map_err(|error| error.code)?;
    let store = Store::open(destination.path(), profile).map_err(|error| error.code)?;
    let snapshot = store
        .snapshot("manual", Duration::from_millis(750))
        .map_err(|error| error.code)?;
    let copied = source::capture(
        &store
            .directory()
            .join(&snapshot.id)
            .join(&snapshot.payload_directory),
    )
    .map_err(|error| error.code)?;
    if copied != before || source::capture(&source).map_err(|error| error.code)? != before {
        return Err(
            "Source changed during inspection; retry after the game has finished saving".into(),
        );
    }
    println!("Temporary backup verified; source unchanged");
    Ok(())
}
