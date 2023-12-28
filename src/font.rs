use std::path::Path;

use anyhow::Context;

// TODO: this should be using woff2 bindings
pub fn ttf_to_woff2(path: &Path) -> anyhow::Result<()> {
    let output = std::process::Command::new("woff2_compress")
        .args([path])
        .output()
        .context("Failed to run `woff2_compress`")?;

    if !output.status.success() {
        anyhow::bail!("woff2_compress error: {output:?}");
    }

    Ok(())
}
