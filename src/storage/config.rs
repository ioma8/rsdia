//! User config: `${XDG_CONFIG_HOME:-~/.config}/rsdia/config.json`

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::export::{is_wrapper, Charset, ExportConfig, Wrapper, DEFAULT_EXPORT};
use crate::storage::drawings::{json_escape, xdg_dir};
use crate::tui::theme::ThemeName;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GridStyle {
    #[default]
    Lattice,
    Checker,
    Dots,
    Off,
}

pub const GRID_STYLES: [GridStyle; 4] = [
    GridStyle::Lattice,
    GridStyle::Checker,
    GridStyle::Dots,
    GridStyle::Off,
];

impl GridStyle {
    pub fn name(self) -> &'static str {
        match self {
            GridStyle::Lattice => "lattice",
            GridStyle::Checker => "checker",
            GridStyle::Dots => "dots",
            GridStyle::Off => "off",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        GRID_STYLES.into_iter().find(|g| g.name() == value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub theme: ThemeName,
    pub grid: GridStyle,
    /// Whether finishing a selection puts it on the clipboard by itself.
    pub copy_on_select: bool,
    pub last_drawing: Option<String>,
    pub export: ExportConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeName::Terminal,
            grid: GridStyle::Lattice,
            copy_on_select: false,
            last_drawing: None,
            export: DEFAULT_EXPORT,
        }
    }
}

pub const DEFAULT_CONFIG: Config = Config {
    theme: ThemeName::Terminal,
    grid: GridStyle::Lattice,
    copy_on_select: false,
    last_drawing: None,
    export: DEFAULT_EXPORT,
};

pub fn config_path() -> PathBuf {
    xdg_dir("XDG_CONFIG_HOME", &[".config"])
        .join("rsdia")
        .join("config.json")
}

pub fn load_config(path: &Path) -> Config {
    let Ok(text) = fs::read_to_string(path) else {
        return DEFAULT_CONFIG;
    };
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) else {
        return DEFAULT_CONFIG;
    };
    let export = raw.get("export");
    Config {
        theme: raw
            .get("theme")
            .and_then(|v| v.as_str())
            .and_then(ThemeName::parse)
            .unwrap_or(DEFAULT_CONFIG.theme),
        grid: raw
            .get("grid")
            .and_then(|v| v.as_str())
            .and_then(GridStyle::parse)
            .unwrap_or(DEFAULT_CONFIG.grid),
        copy_on_select: raw.get("copyOnSelect").and_then(|v| v.as_bool()) == Some(true),
        last_drawing: raw
            .get("lastDrawing")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        export: ExportConfig {
            characters: match export
                .and_then(|e| e.get("characters"))
                .and_then(|v| v.as_str())
            {
                Some("basic") => Charset::Basic,
                _ => Charset::Extended,
            },
            wrapper: export
                .and_then(|e| e.get("wrapper"))
                .and_then(|v| v.as_str())
                .and_then(is_wrapper)
                .unwrap_or(Wrapper::None),
            fenced: export
                .and_then(|e| e.get("fenced"))
                .and_then(|v| v.as_bool())
                == Some(true),
        },
    }
}

/// Config is a convenience; a failure here is never worth crashing the editor over.
pub fn save_config(config: &Config, path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let last = match &config.last_drawing {
        Some(p) => json_escape(p),
        None => "null".to_string(),
    };
    let text = format!(
        "{{\n  \"theme\": \"{}\",\n  \"grid\": \"{}\",\n  \"copyOnSelect\": {},\n  \"lastDrawing\": {},\n  \"export\": {{\n    \"characters\": \"{}\",\n    \"wrapper\": \"{}\",\n    \"fenced\": {}\n  }}\n}}\n",
        config.theme.name(),
        config.grid.name(),
        config.copy_on_select,
        last,
        match config.export.characters {
            Charset::Basic => "basic",
            Charset::Extended => "extended",
        },
        crate::core::export::wrapper_id(config.export.wrapper),
        config.export.fenced,
    );
    let _ = fs::write(path, text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_sanitises() {
        let dir = std::env::temp_dir().join(format!("rsdia-cfg-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.json");
        let _ = fs::remove_file(&path);
        assert_eq!(load_config(&path).grid, GridStyle::Lattice);
        let mut c = load_config(&path);
        c.grid = GridStyle::Checker;
        c.export.wrapper = Wrapper::Hash;
        save_config(&c, &path);
        assert_eq!(load_config(&path), c);
        fs::write(&path, "{\"grid\":\"bogus\",\"theme\":\"gruvbox\"}").expect("writable");
        let bad = load_config(&path);
        assert_eq!(bad.grid, GridStyle::Lattice);
        assert_eq!(bad.theme.name(), "gruvbox");
        let _ = fs::remove_dir_all(&dir);
    }
}
