//! Drawing files: `${XDG_DATA_HOME:-~/.local/share}/rsdia/drawings/<slug>.rd.json`

use std::fs;
use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

use crate::core::layer::Layer;
use crate::core::vector::Pos;

pub const FILE_EXT: &str = ".rd.json";

pub fn xdg_dir(var: &str, fallback: &[&str]) -> PathBuf {
    if let Ok(dir) = std::env::var(var) {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
    for part in fallback {
        path.push(part);
    }
    path
}

pub fn data_dir() -> PathBuf {
    xdg_dir("XDG_DATA_HOME", &[".local", "share"]).join("rsdia")
}

pub fn drawings_dir() -> PathBuf {
    data_dir().join("drawings")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawingInfo {
    pub name: String,
    pub path: PathBuf,
    /// Number of non-empty cells.
    pub size: usize,
}

/// `Café Plan / v2` -> `cafe-plan-v2`; anything without alphanumerics -> `untitled`.
pub fn slugify(name: &str) -> String {
    let decomposed: String = name.nfkd().collect();
    let lowered = decomposed.to_lowercase();
    let mut slug = String::new();
    let mut pending_dash = false;
    for c in lowered.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(c);
        } else {
            pending_dash = true;
        }
    }
    if slug.is_empty() {
        "untitled".to_string()
    } else {
        slug
    }
}

pub fn path_for_name(name: &str, dir: &Path) -> PathBuf {
    dir.join(format!("{}{FILE_EXT}", slugify(name)))
}

/// `JSON.stringify`-compatible string escaping, so drawing files stay byte-identical.
pub(crate) fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Cells sorted row-major so saves are deterministic.
pub fn serialize(name: &str, layer: &Layer) -> String {
    let mut cells: Vec<(Pos, char)> = layer
        .entries()
        .filter(|(_, v)| !crate::core::layer::is_erase(*v))
        .collect();
    cells.sort_by_key(|(p, _)| (p.y, p.x));
    let lines: Vec<String> = cells
        .iter()
        .map(|(p, v)| format!("    [{},{},{}]", p.x, p.y, json_escape(&v.to_string())))
        .collect();
    let cells_field = if cells.is_empty() {
        "  \"cells\": []".to_string()
    } else {
        format!("  \"cells\": [\n{}\n  ]", lines.join(",\n"))
    };
    [
        "{".to_string(),
        "  \"version\": 1,".to_string(),
        format!("  \"name\": {},", json_escape(name)),
        cells_field,
        "}".to_string(),
        String::new(),
    ]
    .join("\n")
}

/// Returns the drawing's name and cells. `createdAt`/`updatedAt` in files written
/// by older versions are ignored.
pub fn deserialize(text: &str) -> Result<(String, Layer), String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("Unsupported drawing file: {e}"))?;
    let version = value.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
    let cells = value.get("cells").and_then(|v| v.as_array());
    let (Some(cells), 1) = (cells, version) else {
        return Err("Unsupported drawing file".to_string());
    };
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let mut layer = Layer::new();
    for cell in cells {
        let Some(items) = cell.as_array() else {
            continue;
        };
        let (Some(x), Some(y), Some(v)) = (
            items.first().and_then(|v| v.as_i64()),
            items.get(1).and_then(|v| v.as_i64()),
            items.get(2).and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        let Some(ch) = v.chars().next() else { continue };
        layer.set(Pos::new(x as i32, y as i32), ch);
    }
    Ok((name, layer))
}

pub struct DrawingStore {
    pub dir: PathBuf,
}

impl Default for DrawingStore {
    fn default() -> Self {
        Self {
            dir: drawings_dir(),
        }
    }
}

impl DrawingStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn ensure_dir(&self) {
        let _ = fs::create_dir_all(&self.dir);
    }

    pub fn list(&self) -> Vec<DrawingInfo> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.to_string_lossy().ends_with(FILE_EXT) {
                continue;
            }
            // Skip unreadable files rather than failing the whole list.
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok((name, layer)) = deserialize(&text) {
                    out.push(DrawingInfo {
                        name,
                        path,
                        size: layer.len(),
                    });
                }
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub fn exists(&self, name: &str) -> bool {
        let slug = slugify(name);
        self.list().iter().any(|d| slugify(&d.name) == slug)
    }

    /// "untitled", "untitled 2", ... — first name not taken.
    pub fn unique_name(&self, base: &str) -> String {
        if !self.exists(base) {
            return base.to_string();
        }
        let mut i = 2;
        loop {
            let candidate = format!("{base} {i}");
            if !self.exists(&candidate) {
                return candidate;
            }
            i += 1;
        }
    }

    pub fn path_for(&self, name: &str) -> PathBuf {
        path_for_name(name, &self.dir)
    }

    pub fn load(&self, path: &Path) -> Result<(String, Layer), String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        deserialize(&text)
    }

    /// Atomic write: temp file + rename.
    pub fn save(&self, path: &Path, name: &str, layer: &Layer) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let tmp = path.with_file_name(format!(
            ".{}.tmp",
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        fs::write(&tmp, serialize(name, layer)).map_err(|e| e.to_string())?;
        fs::rename(&tmp, path).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn rename(
        &self,
        old_path: &Path,
        new_name: &str,
        layer: &Layer,
    ) -> Result<PathBuf, String> {
        let new_path = self.path_for(new_name);
        self.save(&new_path, new_name, layer)?;
        if new_path != old_path {
            let _ = fs::remove_file(old_path);
        }
        Ok(new_path)
    }

    pub fn delete(&self, path: &Path) {
        let _ = fs::remove_file(path);
    }

    pub fn create(&self, name: &str) -> Result<PathBuf, String> {
        self.ensure_dir();
        let path = self.path_for(name);
        self.save(&path, name, &Layer::new())?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::text::text_to_layer;

    #[test]
    fn slugify_matches_asciiflow_names() {
        assert_eq!(slugify("  Café Plan / v2 "), "cafe-plan-v2");
        assert_eq!(slugify("***"), "untitled");
        assert_eq!(slugify("Big Diagram!"), "big-diagram");
    }

    #[test]
    fn save_load_round_trip_is_byte_identical() {
        let layer = text_to_layer("┌──┐\n│ \"x\" │\n└──┘ ☃", Pos::default());
        let text = serialize("round trip", &layer);
        let (name, loaded) = deserialize(&text).expect("parses");
        assert_eq!(serialize(&name, &loaded), text);
        assert!(text.contains("  \"cells\": ["));
    }

    #[test]
    fn files_with_timestamps_load_and_are_rewritten_without_them() {
        let legacy = r#"{
  "version": 1,
  "name": "old",
  "createdAt": "2026-01-01T00:00:00.000Z",
  "updatedAt": "2026-01-02T00:00:00.000Z",
  "cells": [
    [0,0,"┌"],[1,0,"┐"],
    [0,1,"└"],[1,1,"┘"]
  ]
}"#;
        let (name, layer) = deserialize(legacy).expect("older files still parse");
        assert_eq!(name, "old");
        assert_eq!(layer.len(), 4);
        let rewritten = serialize(&name, &layer);
        assert!(!rewritten.contains("createdAt"));
        assert!(!rewritten.contains("updatedAt"));
    }

    #[test]
    fn erase_markers_are_not_written() {
        let mut layer = Layer::new();
        layer.set(Pos::new(0, 0), 'a');
        layer.set(Pos::new(1, 0), crate::core::layer::ERASE);
        let text = serialize("t", &layer);
        assert!(text.contains("[0,0,\"a\"]"));
        assert_eq!(deserialize(&text).expect("parses").1.len(), 1);
    }
}
