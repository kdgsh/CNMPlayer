use anyhow::{Context, Result};
use directories::BaseDirs;
use std::fs;
use std::path::{Path, PathBuf};

const ENV_ASSET_DIR: &str = "CNMPLAYER_ASSET_DIR";

const DEFAULT_CONFIG_TOML: &str = include_str!("../../config/default.toml");

const THEME_SYSTEM_TOML: &str = include_str!("../../themes/system.toml");
const THEME_LATTE_TOML: &str = include_str!("../../themes/catppuccin_latte.toml");
const THEME_FRAPPE_TOML: &str = include_str!("../../themes/catppuccin_frappe.toml");
const THEME_MACCHIATO_TOML: &str = include_str!("../../themes/catppuccin_macchiato.toml");
const THEME_MOCHA_TOML: &str = include_str!("../../themes/catppuccin_mocha.toml");

pub fn resolve_asset_root() -> PathBuf {
    if let Some(path) = std::env::var_os(ENV_ASSET_DIR) {
        return PathBuf::from(path);
    }

    if let Some(system_root) = system_config_root() {
        let _ = migrate_legacy_local_assets(&system_root);
        let _ = ensure_all_assets(&system_root);
        return system_root;
    }

    let fallback = legacy_local_root();
    let _ = ensure_all_assets(&fallback);
    fallback
}

pub fn resolve_asset_path(rel: &Path) -> PathBuf {
    resolve_asset_root().join(rel)
}

pub fn resolve_config_path() -> PathBuf {
    resolve_asset_path(Path::new("config/default.toml"))
}

pub fn ensure_assets_ready() -> Result<PathBuf> {
    if let Some(system_root) = system_config_root() {
        let _ = migrate_legacy_local_assets(&system_root);
        ensure_all_assets(&system_root)?;
        return Ok(system_root);
    }

    let fallback = legacy_local_root();
    ensure_all_assets(&fallback)?;
    Ok(fallback)
}

fn system_config_root() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.config_dir().join("cnmplayer"))
}

fn legacy_local_root() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".config")
        .join("cnmplayer")
}

fn migrate_legacy_local_assets(system_root: &Path) -> Result<()> {
    let legacy_root = legacy_local_root();
    let legacy_cfg = legacy_root.join("config/default.toml");
    let system_cfg = system_root.join("config/default.toml");

    if system_cfg.is_file() || !legacy_cfg.is_file() {
        return Ok(());
    }

    ensure_dir(&system_root.join("config"))?;
    ensure_dir(&system_root.join("themes"))?;

    fs::copy(&legacy_cfg, &system_cfg)
        .with_context(|| format!("copy {} -> {}", legacy_cfg.display(), system_cfg.display()))?;

    let legacy_themes = legacy_root.join("themes");
    if legacy_themes.is_dir() {
        for entry in fs::read_dir(&legacy_themes)
            .with_context(|| format!("read_dir {}", legacy_themes.display()))?
        {
            let entry = entry?;
            let source = entry.path();
            if source.is_file() {
                if let Some(name) = source.file_name() {
                    let target = system_root.join("themes").join(name);
                    let _ = fs::copy(&source, &target);
                }
            }
        }
    }

    Ok(())
}

fn ensure_all_assets(root: &Path) -> Result<()> {
    ensure_dir(&root.join("config"))?;
    ensure_dir(&root.join("themes"))?;

    write_if_missing(&root.join("config/default.toml"), DEFAULT_CONFIG_TOML)?;
    ensure_themes(root)?;

    Ok(())
}

fn ensure_themes(root: &Path) -> Result<()> {
    ensure_dir(&root.join("themes"))?;

    write_if_missing(&root.join("themes/system.toml"), THEME_SYSTEM_TOML)?;
    write_if_missing(&root.join("themes/catppuccin_latte.toml"), THEME_LATTE_TOML)?;
    write_if_missing(&root.join("themes/catppuccin_frappe.toml"), THEME_FRAPPE_TOML)?;
    write_if_missing(
        &root.join("themes/catppuccin_macchiato.toml"),
        THEME_MACCHIATO_TOML,
    )?;
    write_if_missing(&root.join("themes/catppuccin_mocha.toml"), THEME_MOCHA_TOML)?;

    Ok(())
}

fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("mkdir {}", path.display()))
}

fn write_if_missing(path: &Path, contents: &str) -> Result<()> {
    if path.is_file() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}
