use std::path::PathBuf;

use slint_build::CompilerConfiguration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    compile_locales()?;
    let root = env!("CARGO_MANIFEST_DIR");
    let mut main = PathBuf::from(root);
    main.push("ui/main_window.slint");

    let mut include = PathBuf::from(root);
    include.push("ui");

    slint_build::print_rustc_flags()?;
    slint_build::compile_with_config(
        main,
        CompilerConfiguration::new()
            // .embed_resources(EmbedResourcesKind::EmbedFiles)
            .with_include_paths(vec![include])
            .with_style("fluent".into()),
    )?;
    Ok(())
}

/// Compile the .po sources into gettext catalogs under `OUT_DIR`, and export
/// that location as `ROGCC_TRANSLATIONS_DIR` for the runtime to load from.
fn compile_locales() -> Result<(), Box<dyn std::error::Error>> {
    let sources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("translations");
    let out = PathBuf::from(std::env::var("OUT_DIR")?).join("translations");

    println!("cargo:rustc-env=ROGCC_TRANSLATIONS_DIR={}/", out.display());
    // Catches locales being added or removed, not just edited
    println!("cargo:rerun-if-changed={}", sources.display());

    std::fs::create_dir_all(&out)?;

    let mut compiled = Vec::new();
    for entry in std::fs::read_dir(&sources)? {
        let entry = entry?;
        let po = entry.path().join("rog-control-center.po");
        // `en` is the extraction template and every msgstr is empty in it
        if entry.file_name() == "en" || !po.exists() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", po.display());

        let dir = out.join(entry.file_name()).join("LC_MESSAGES");
        std::fs::create_dir_all(&dir)?;
        let mo = dir.join("rog-control-center.mo");

        let status = std::process::Command::new("msgfmt")
            .arg("-o")
            .arg(&mo)
            .arg(&po)
            .status()
            .map_err(|e| format!("could not run msgfmt, is gettext installed? ({e})"))?;
        if !status.success() {
            return Err(format!("msgfmt failed for {} ({status})", po.display()).into());
        }
        compiled.push(entry.file_name());
    }

    // Cargo never clears OUT_DIR, so catalogs of removed locales would linger
    // and still be picked up by the install step
    for entry in std::fs::read_dir(&out)? {
        let entry = entry?;
        if !compiled.contains(&entry.file_name()) {
            std::fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}
