fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 2 {
        return Err("Usage: prepare_effects SOURCE DESTINATION".into());
    }
    arachnid_mods::effects::deploy(
        std::path::Path::new(&arguments[0]),
        std::path::Path::new(&arguments[1]),
    )
    .map_err(|e| e.code)?;
    println!(
        "Local effects and dependencies verified and prepared; no shader or game was executed."
    );
    Ok(())
}
