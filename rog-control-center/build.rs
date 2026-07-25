use std::path::PathBuf;

use slint_build::CompilerConfiguration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // write_locales();
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
