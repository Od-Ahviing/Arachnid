fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let catalog = arachnid_mods::bundled_catalog().map_err(|error| error.code)?;
    let index = arachnid_mods::offline::bundled_index(&catalog).map_err(|error| error.code)?;
    let mode = arguments
        .first()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    match (mode, arguments.len()) {
        ("inspect", 2) => {
            let report = index
                .inspect(std::path::Path::new(&arguments[1]))
                .map_err(|error| error.code)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        ("stage", 3) => {
            let report = index
                .stage_available(
                    std::path::Path::new(&arguments[1]),
                    std::path::Path::new(&arguments[2]),
                )
                .map_err(|error| error.code)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        ("verify", 2) => {
            index
                .verify_release(&catalog, std::path::Path::new(&arguments[1]))
                .map_err(|error| error.code)?;
            arachnid_mods::performance::verify_release(
                std::path::Path::new(&arguments[1])
                    .parent()
                    .ok_or("missing resource parent")?
                    .join(arachnid_mods::performance::RESOURCE_DIRECTORY)
                    .as_path(),
            )
            .map_err(|error| error.code)?;
            arachnid_mods::effects::verify_prepared(
                &std::path::Path::new(&arguments[1])
                    .parent()
                    .ok_or("missing resource parent")?
                    .join("reshade-effects"),
            )
            .map_err(|error| error.code)?;
            arachnid_mods::skins::verify_release(
                &std::path::Path::new(&arguments[1])
                    .parent()
                    .ok_or("missing resource parent")?
                    .join(arachnid_mods::skins::RESOURCE_DIRECTORY),
            )
            .map_err(|error| error.code)?;
        }
        _ => {
            return Err(
                "Usage: offline inspect DIRECTORY | stage SOURCE DESTINATION | verify DIRECTORY"
                    .into(),
            );
        }
    }
    Ok(())
}
