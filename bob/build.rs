#[cfg(feature = "schema")]
fn build_mangen() -> std::io::Result<()> {
    use clap_builder::CommandFactory;

    let out_dir =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").ok_or(std::io::ErrorKind::NotFound)?);

    let cmd = bob_cli::Cli::command();
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(out_dir.join("bob.1"), buffer)?;

    for subcommand in cmd.get_subcommands() {
        let man = clap_mangen::Man::new(subcommand.clone());
        let mut buf = Vec::new();
        man.render(&mut buf).expect("rendering subcommand worked");

        let name = format!("bob-{}.1", subcommand.get_name());
        std::fs::write(out_dir.join(name), buf)?;
    }

    Ok(())
}

#[cfg(feature = "doc")]
fn add_docs_imgs() {
    use std::path::PathBuf;

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("missing cargo manifest env");
    let docs = PathBuf::from(&crate_dir)
        .join("..")
        .join("target")
        .join("doc")
        .canonicalize()
        .expect("invalid docs path");
    let imgs = docs.join("img");

    let logo = PathBuf::from(&crate_dir)
        .join("doc")
        .join("img")
        .join("logo.png");
    std::fs::create_dir_all(&imgs).expect("failed to make docs images dir");
    std::fs::copy(logo, imgs.join("logo.png")).expect("failed to copy image");
}

fn main() -> std::io::Result<()> {
    #[cfg(feature = "schema")]
    build_mangen()?;

    #[cfg(feature = "doc")]
    add_docs_imgs();

    Ok(())
}
