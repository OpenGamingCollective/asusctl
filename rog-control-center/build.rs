use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use slint_build::CompilerConfiguration;

fn compile_translations(manifest_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let translations_dir = manifest_dir.join("translations");
    if !translations_dir.exists() {
        return Ok(());
    }

    println!("cargo:rerun-if-changed={}", translations_dir.display());

    for entry in fs::read_dir(&translations_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let po_file = path.join("rog-control-center.po");
            let lc_messages_dir = path.join("LC_MESSAGES");
            let mo_file = lc_messages_dir.join("rog-control-center.mo");

            if po_file.exists() {
                println!("cargo:rerun-if-changed={}", po_file.display());
                println!("cargo:rerun-if-changed={}", mo_file.display());
                fs::create_dir_all(&lc_messages_dir)?;

                let status = Command::new("msgfmt")
                    .arg(&po_file)
                    .arg("-o")
                    .arg(&mo_file)
                    .status()
                    .map_err(|e| format!("Failed to execute msgfmt: {}", e))?;

                if !status.success() {
                    return Err(format!(
                        "msgfmt failed for {}: exit status {}",
                        po_file.display(),
                        status
                    )
                    .into());
                }
            } else if mo_file.exists() {
                fs::remove_file(&mo_file)?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    compile_translations(&root)?;

    let mut main = root.clone();
    main.push("ui/main_window.slint");

    let mut include = root.clone();
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
