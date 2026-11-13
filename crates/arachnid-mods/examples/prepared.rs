use arachnid_mods::{effects, performance, skins};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let directory = arguments
        .next()
        .ok_or("Usage: prepared DIRECTORY [--release]")?;
    let release = match arguments.next() {
        None => false,
        Some(flag) if flag == "--release" && arguments.next().is_none() => true,
        _ => return Err("Usage: prepared DIRECTORY [--release]".into()),
    };
    let root = Path::new(&directory);
    let [verify_performance, verify_skins]: [fn(&Path) -> _; 2] = if release {
        [performance::verify_release, skins::verify_release]
    } else {
        [performance::verify_prepared, skins::verify_prepared]
    };
    verify_performance(&root.join(performance::RESOURCE_DIRECTORY)).map_err(|error| error.code)?;
    effects::verify_prepared(&root.join("reshade-effects")).map_err(|error| error.code)?;
    verify_skins(&root.join(skins::RESOURCE_DIRECTORY)).map_err(|error| error.code)?;
    println!(
        "Prepared resources verified: seven graphics compositions, Vulkan layer, shaders and 22 texture packages. No network request or game mutation."
    );
    Ok(())
}
