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

pub fn now_iso() -> String {
    // `Date.toISOString()` shape: UTC with milliseconds and a Z suffix.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let days = (secs / 86_400) as i64;
    let time = secs % 86_400;
    let (h, m, s) = (time / 3600, (time % 3600) / 60, time % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

/// Howard Hinnant's days-to-civil conversion.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawingFile {
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub cells: Vec<(i32, i32, char)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawingInfo {
    pub name: String,
    pub path: PathBuf,
    /// Number of non-empty cells.
    pub size: usize,
    pub updated_at: String,
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
pub fn serialize(name: &str, layer: &Layer, created_at: &str, updated_at: &str) -> String {
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
        format!("  \"createdAt\": {},", json_escape(created_at)),
        format!("  \"updatedAt\": {},", json_escape(updated_at)),
        cells_field,
        "}".to_string(),
        String::new(),
    ]
    .join("\n")
}

pub fn deserialize(text: &str) -> Result<(DrawingFile, Layer), String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("Unsupported drawing file: {e}"))?;
    let version = value.get("version").and_then(|v| v.as_i64()).unwrap_or(0);
    let cells = value.get("cells").and_then(|v| v.as_array());
    let (Some(cells), 1) = (cells, version) else {
        return Err("Unsupported drawing file".to_string());
    };
    let mut layer = Layer::new();
    let mut file = DrawingFile {
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        created_at: value
            .get("createdAt")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        updated_at: value
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        cells: Vec::new(),
    };
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
        file.cells.push((x as i32, y as i32, ch));
    }
    Ok((file, layer))
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
                if let Ok((file, _)) = deserialize(&text) {
                    out.push(DrawingInfo {
                        name: file.name,
                        path,
                        size: file.cells.len(),
                        updated_at: file.updated_at,
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

    pub fn load(&self, path: &Path) -> Result<(DrawingFile, Layer), String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        deserialize(&text)
    }

    /// Atomic write: temp file + rename. Returns the new `updatedAt`.
    pub fn save(
        &self,
        path: &Path,
        name: &str,
        layer: &Layer,
        created_at: &str,
    ) -> Result<String, String> {
        let updated_at = now_iso();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let tmp = path.with_file_name(format!(
            ".{}.tmp",
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        fs::write(&tmp, serialize(name, layer, created_at, &updated_at))
            .map_err(|e| e.to_string())?;
        fs::rename(&tmp, path).map_err(|e| e.to_string())?;
        Ok(updated_at)
    }

    pub fn rename(
        &self,
        old_path: &Path,
        new_name: &str,
        layer: &Layer,
        created_at: &str,
    ) -> Result<PathBuf, String> {
        let new_path = self.path_for(new_name);
        self.save(&new_path, new_name, layer, created_at)?;
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
        self.save(&path, name, &Layer::new(), &now_iso())?;
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
        let text = serialize(
            "round trip",
            &layer,
            "2026-01-01T00:00:00.000Z",
            "2026-01-02T00:00:00.000Z",
        );
        let (file, loaded) = deserialize(&text).expect("parses");
        assert_eq!(
            serialize(&file.name, &loaded, &file.created_at, &file.updated_at),
            text
        );
        assert!(text.contains("  \"cells\": ["));
    }

    #[test]
    fn erase_markers_are_not_written() {
        let mut layer = Layer::new();
        layer.set(Pos::new(0, 0), 'a');
        layer.set(Pos::new(1, 0), crate::core::layer::ERASE);
        let text = serialize("t", &layer, "c", "u");
        assert!(text.contains("[0,0,\"a\"]"));
        assert_eq!(deserialize(&text).expect("parses").1.len(), 1);
    }
}
