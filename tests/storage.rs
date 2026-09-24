//! Drawing store, config, and the `export` CLI's observable contract.

use std::path::PathBuf;
use std::process::Command;

use rsdia::core::export::{export_text, Charset, ExportConfig, Wrapper};
use rsdia::core::text::text_to_layer;
use rsdia::core::vector::Pos;
use rsdia::storage::config::{load_config, save_config, GridStyle};
use rsdia::storage::drawings::{deserialize, serialize, slugify, DrawingStore};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rsdia-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a writable temp dir");
    dir
}

#[test]
fn save_load_round_trip_is_byte_identical() {
    let layer = text_to_layer("┌──┐\n│ \"x\" │\n└──┘ ☃", Pos::default());
    let text = serialize("round trip", &layer);
    let (name, loaded) = deserialize(&text).expect("parses");
    assert_eq!(serialize(&name, &loaded), text);
    assert_eq!(
        rsdia::core::text::layer_to_text(&loaded, None, true),
        "┌──┐\n│ \"x\" │\n└──┘ ☃"
    );
}

#[test]
fn the_store_creates_lists_renames_and_deletes() {
    let dir = temp_dir("store");
    let store = DrawingStore::new(dir.join("store"));
    assert!(store.list().is_empty());
    let a = store.create("untitled").expect("created");
    assert_eq!(store.unique_name("untitled"), "untitled 2");
    store
        .save(&a, "untitled", &text_to_layer("hello", Pos::default()))
        .expect("saved");
    let listed = store.list();
    assert_eq!(listed.len(), 1);
    assert_eq!((listed[0].name.as_str(), listed[0].size), ("untitled", 5));
    let layer = store.load(&a).expect("loads").1;
    let b = store.rename(&a, "Big Diagram!", &layer).expect("renamed");
    assert!(b.to_string_lossy().ends_with("big-diagram.rd.json"));
    assert!(!store.exists("untitled"));
    assert!(store.exists("big diagram"));
    store.delete(&b);
    assert!(store.list().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn slugify_strips_accents_and_symbols() {
    assert_eq!(slugify("  Café Plan / v2 "), "cafe-plan-v2");
    assert_eq!(slugify("***"), "untitled");
}

#[test]
fn config_round_trips_and_sanitises() {
    let dir = temp_dir("config");
    let path = dir.join("config.json");
    assert_eq!(load_config(&path).grid, GridStyle::Lattice);
    let mut c = load_config(&path);
    c.grid = GridStyle::Checker;
    c.export.wrapper = Wrapper::Hash;
    save_config(&c, &path);
    assert_eq!(load_config(&path), c);
    std::fs::write(&path, "{\"grid\":\"bogus\",\"theme\":\"gruvbox\"}").expect("writable");
    let bad = load_config(&path);
    assert_eq!(bad.grid, GridStyle::Lattice);
    assert_eq!(bad.theme.name(), "gruvbox");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_export_command_writes_what_the_library_would() {
    let dir = temp_dir("cli");
    let data = dir.join("cli");
    let store = DrawingStore::new(data.join("rsdia").join("drawings"));
    let layer = text_to_layer("┌┐\n└┘", Pos::default());
    let path = store.path_for("cli test");
    store.save(&path, "cli test", &layer).expect("saved");

    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rsdia"))
            .args(args)
            .env("XDG_DATA_HOME", &data)
            .env("XDG_CONFIG_HOME", dir.join("cli-config"))
            .output()
            .expect("the binary runs")
    };

    let plain = run(&["export", "cli test"]);
    assert!(plain.status.success());
    assert_eq!(String::from_utf8_lossy(&plain.stdout), "┌┐\n└┘\n");

    let out = dir.join("out.txt");
    let basic = run(&[
        "export",
        "cli test",
        "--basic",
        "--comment",
        "hashes",
        "--fenced",
        "-o",
        out.to_str().expect("utf-8 path"),
    ]);
    assert!(basic.status.success());
    let expected = export_text(
        &layer,
        &ExportConfig {
            characters: Charset::Basic,
            wrapper: Wrapper::Hash,
            fenced: true,
        },
    );
    assert_eq!(
        std::fs::read_to_string(&out).expect("written"),
        format!("{expected}\n")
    );

    let missing = run(&["export", "nope"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("no drawing named \"nope\""));

    let version = run(&["--version"]);
    let version = String::from_utf8_lossy(&version.stdout).trim().to_string();
    assert_eq!(version.split('.').count(), 3, "a semver: {version}");

    let list = run(&["list"]);
    assert!(String::from_utf8_lossy(&list.stdout).contains("cli test"));

    // A drawing written before the rename still opens when it is named.
    let legacy = data
        .join("lazydraw")
        .join("drawings")
        .join("old-name.ld.json");
    std::fs::create_dir_all(legacy.parent().expect("a parent")).expect("created");
    std::fs::copy(&path, &legacy).expect("copied");
    let opened = run(&["export", legacy.to_str().expect("utf-8 path")]);
    assert!(opened.status.success());
    assert_eq!(String::from_utf8_lossy(&opened.stdout), "┌┐\n└┘\n");
    let _ = std::fs::remove_dir_all(&dir);
}
