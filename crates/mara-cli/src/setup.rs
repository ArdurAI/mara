//! `mara setup <preset>` — write a starter configuration for a runtime.

use std::path::{Path, PathBuf};

use anyhow::Context;

/// Minimal runnable config: Ollama HTTP proxy on `11435` → upstream `11434`.
const OLLAMA_PRESET_TOML: &str = r#"schema_version = "1"

[server]
metrics_addr = "127.0.0.1:9099"
log_format = "text"

[[adapters.llm_proxy]]
name = "ollama_proxy"
http_listen = "127.0.0.1:11435"
upstream = "http://127.0.0.1:11434"
normalizer = "ollama"

[[sinks.stdout]]
name = "default_out"
pretty = true

[[pipelines]]
name = "ollama"
adapters = ["ollama_proxy"]
policy_chain = "default"
sinks = ["default_out"]
"#;

/// Apply a named setup preset.
pub fn setup(preset: &str, force: bool) -> anyhow::Result<()> {
    match preset {
        "ollama" => setup_ollama(force),
        _ => anyhow::bail!("unknown preset {preset:?}; supported presets: ollama"),
    }
}

fn config_path_under_home(home: &Path) -> PathBuf {
    let dir = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/mara")
    } else {
        home.join(".config/mara")
    };
    dir.join("mara.toml")
}

fn default_mara_config_path() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set; cannot choose a config path")?;
    Ok(config_path_under_home(Path::new(&home)))
}

fn write_atomic(path: &Path, contents: &str) -> anyhow::Result<()> {
    let parent = path.parent().context("config path has no parent directory")?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("temp file in {}", parent.display()))?;
    std::io::Write::write_all(&mut tmp, contents.as_bytes())
        .with_context(|| format!("write {}", tmp.path().display()))?;
    tmp.persist(path).map_err(|e| anyhow::anyhow!("install config: {e}"))?;
    Ok(())
}

fn setup_ollama(force: bool) -> anyhow::Result<()> {
    setup_ollama_to_path(&default_mara_config_path()?, force)
}

fn setup_ollama_to_path(path: &Path, force: bool) -> anyhow::Result<()> {
    if path.exists() && !force {
        anyhow::bail!("refusing to overwrite {} (use --force to replace)", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    write_atomic(path, OLLAMA_PRESET_TOML)?;
    mara_core::Config::from_file(path)
        .with_context(|| format!("validate written {}", path.display()))?;

    println!("Wrote {}", path.display());
    println!();
    println!("Run: mara run --config {}", path.display());
    println!();
    println!(
        "Point HTTP clients at the proxy (see `http_listen` in [[adapters.llm_proxy]]), not directly at Ollama, so Mara can emit events."
    );
    println!();
    if cfg!(target_os = "macos") {
        println!("If you need Ollama itself on a different port, use:");
        println!("{}", mara_runtime_ollama::suggested_macos_reconfig());
    } else if cfg!(unix) {
        println!("If you need Ollama itself on a different port, use:");
        println!("{}", mara_runtime_ollama::suggested_linux_reconfig());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_preset_parses() {
        mara_core::Config::from_toml_str(OLLAMA_PRESET_TOML, "preset").expect("valid");
    }

    #[test]
    fn setup_ollama_writes_valid_config() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("mara.toml");
        setup_ollama_to_path(&path, false).expect("setup");
        let raw = std::fs::read_to_string(&path).expect("read");
        assert!(raw.contains("ollama_proxy"));
        mara_core::Config::from_file(&path).expect("round-trip validate");
    }
}
