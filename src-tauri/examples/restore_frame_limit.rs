fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 2 {
        return Err("Usage: restore_frame_limit GAME_DIRECTORY ARACHNID_DATA_DIRECTORY".into());
    }
    let game = arachnid_detector::validate_root(std::path::Path::new(&arguments[0]))
        .map_err(|e| e.code)?;
    let data = std::path::Path::new(&arguments[1]);
    let lease = arachnid_core::coordination::InstallationLease::acquire(&game, &data.join("locks"))
        .map_err(|e| e.code)?;
    let state = data.join("mods").join(lease.id()).join("frame-limit");
    arachnid_mods::frame_limit::restore(lease.root(), &state, &lease.id(), || {
        arachnid_launch::ensure_game_stopped(lease.root())
    })
    .map_err(|e| e.code)?;
    println!(
        "Original executable restored from verified backup. The automatic frame limit patch has been withdrawn."
    );
    Ok(())
}
