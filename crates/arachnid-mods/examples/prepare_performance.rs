fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "usage: prepare_performance FIX_ZIP DXVK_X32_DLL RESHADE_SETUP PRIVATE_DESTINATION"
                .into(),
        );
    }
    arachnid_mods::performance::prepare_preview(
        std::path::Path::new(&args[0]),
        std::path::Path::new(&args[1]),
        std::path::Path::new(&args[2]),
        std::path::Path::new(&args[3]),
    )
    .map_err(|error| error.code)?;
    println!("Private performance candidate prepared and verified. Nothing installed or executed.");
    Ok(())
}
