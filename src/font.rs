use std::path::Path;

// TODO: this should be using woff2 bindings
pub fn ttf_to_woff2(path: &Path) -> anyhow::Result<()> {
    let output = std::process::Command::new("woff2_compress")
        .args([path])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("woff2_compress error: {output:?}");
    }

    Ok(())
}
