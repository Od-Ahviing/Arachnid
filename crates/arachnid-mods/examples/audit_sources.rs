use arachnid_mods::{
    Artifact, Preimage, RecipeFile,
    archive::{extract_reshade_setup, extract_zip},
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: audit_sources FIX_ZIP RESHADE_SETUP".into());
    }
    let staging = tempfile::tempdir()?;
    for (index, (url, hash, bytes, expanded, entry, output, payload_hash, payload_bytes)) in [
        (
            "https://community.pcgamingwiki.com/files/file/3782-the-amazing-spider-man-fix/",
            "d672dc66bf25e6bee17736f5764e80517d73c665326f18b31d44c3817b2f382f",
            3_081_564,
            10_000_000,
            "The Amazing Spider-Man Fix/d3d9.dll",
            "fix.dll",
            "738e1c71cee12f7175bf9e4cccc6ec36e3b7104b0ba203328144c5ac85469999",
            2_079_232,
        ),
        (
            "https://reshade.me/downloads/ReShade_Setup_6.6.2.exe",
            "807e145eab98903b20f1fc743598a74e3a3be7394ab015b7f64231228168f04e",
            3_940_880,
            16_000_000,
            "ReShade32.dll",
            "ReShade32.dll",
            "189e265f72ad80d7cc227f5de7c8a29cf32d6a2c90a4a03e5878e5ca0f107ee9",
            3_910_424,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let artifact = Artifact {
            url: url.into(),
            sha256: hash.into(),
            archive_bytes: bytes,
            extracted_bytes: expanded,
        };
        let file = RecipeFile {
            source: entry.into(),
            target: output.into(),
            sha256: payload_hash.into(),
            bytes: payload_bytes,
            preimage: Preimage::Absent,
        };
        let extractor = if index == 0 {
            extract_zip
        } else {
            extract_reshade_setup
        };
        let prepared = extractor(Path::new(&args[index]), &artifact, &[file], staging.path())
            .map_err(|e| e.code)?;
        let file = prepared.file(output).ok_or("missing prepared file")?;
        if index == 0 {
            let original = std::fs::read(&file)?;
            let adapted = arachnid_mods::fix_adapter::adapt(&original).map_err(|e| e.code)?;
            if arachnid_mods::fix_adapter::restore(&adapted).map_err(|e| e.code)? != original {
                return Err("Fix adaptation reversal failed".into());
            }
            let changed = original
                .iter()
                .zip(&adapted)
                .filter(|(a, b)| a != b)
                .count();
            println!(
                "Private in-memory Fix candidate: {}, {changed} changed bytes; not executed or installed.",
                arachnid_mods::fix_adapter::digest(&adapted)
            );
        }
        println!(
            "Verified static extraction: {output}, {} bytes. No module was executed or installed.",
            std::fs::metadata(file)?.len()
        );
    }
    Ok(())
}
