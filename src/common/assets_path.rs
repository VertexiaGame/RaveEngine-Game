use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static CACHED_ASSETS_DIR: OnceLock<PathBuf> = OnceLock::new();

fn is_valid_assets_dir(path: &Path) -> bool {
    path.join("content").exists() || path.join("shaders").exists() || path.join("maps").exists()
}

fn discover_assets_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("VERTIGO_ASSETS_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(mut cur) = exe.parent().map(PathBuf::from) {
            for _ in 0..7 {
                let candidate = cur.join("assets");
                if candidate.is_dir() && is_valid_assets_dir(&candidate) {
                    return candidate;
                }
                match cur.parent() {
                    Some(parent) => cur = parent.to_path_buf(),
                    None => break,
                }
            }
            if let Some(parent) = exe.parent() {
                let fallback = parent.join("assets");
                if fallback.is_dir() {
                    return fallback;
                }
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    if manifest.is_dir() {
        return manifest;
    }

    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("assets");
        if candidate.is_dir() {
            return candidate;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.join("assets");
        }
    }

    PathBuf::from("assets")
}

pub fn assets_dir() -> PathBuf {
    CACHED_ASSETS_DIR
        .get_or_init(discover_assets_dir)
        .clone()
}

pub fn assets_dir_string() -> String {
    assets_dir().to_string_lossy().into_owned()
}

pub fn resolve_asset_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    if p.exists() {
        if let Ok(canon) = p.canonicalize() {
            return canon;
        }
        return p.to_path_buf();
    }
    let trimmed = path
        .strip_prefix("assets/")
        .or_else(|| path.strip_prefix("assets\\"))
        .unwrap_or(path)
        .trim_start_matches(['/', '\\']);
    let candidate = assets_dir().join(trimmed);
    if candidate.exists() {
        return candidate;
    }
    let cwd_candidate = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(path);
    if cwd_candidate.exists() {
        return cwd_candidate;
    }
    let cwd_trimmed = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(trimmed);
    if cwd_trimmed.exists() {
        return cwd_trimmed;
    }
    candidate
}

pub fn resolve_vrtx_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    if p.exists() {
        return p.to_path_buf();
    }
    let trimmed = path
        .strip_prefix("assets/")
        .or_else(|| path.strip_prefix("assets\\"))
        .unwrap_or(path);
    let assets_candidate = assets_dir().join(trimmed);
    if assets_candidate.exists() {
        return assets_candidate;
    }
    let alt = assets_dir().join(path);
    if alt.exists() {
        return alt;
    }
    let cwd_candidate = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(path);
    if cwd_candidate.exists() {
        return cwd_candidate;
    }
    assets_candidate
}

pub fn asset_plugin() -> bevy::asset::AssetPlugin {
    bevy::asset::AssetPlugin {
        file_path: assets_dir_string(),
        ..Default::default()
    }
}

pub fn resolve_launch_info_path() -> PathBuf {
    let primary = resolve_vrtx_path("launch_info.json");
    if primary.exists() {
        return primary;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let exe_candidate = parent.join("launch_info.json");
            if exe_candidate.exists() {
                return exe_candidate;
            }
            if let Some(grand) = parent.parent() {
                let alt = grand.join("launch_info.json");
                if alt.exists() {
                    return alt;
                }
            }
        }
    }
    PathBuf::from("launch_info.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_dir_is_absolute_or_relative() {
        let dir = assets_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn resolve_asset_path_strips_assets_prefix() {
        let resolved = resolve_asset_path("assets/content/game/fonts/Ubuntu.ttf");
        let expected_suffix = Path::new("content/game/fonts/Ubuntu.ttf");
        assert!(resolved.ends_with(expected_suffix));
    }

    #[test]
    fn resolve_asset_path_keeps_absolute() {
        #[cfg(windows)]
        {
            let abs = "C:\\tmp\\file.txt";
            assert_eq!(resolve_asset_path(abs), PathBuf::from(abs));
        }
        #[cfg(not(windows))]
        {
            let abs = "/tmp/file.txt";
            assert_eq!(resolve_asset_path(abs), PathBuf::from(abs));
        }
    }
}
