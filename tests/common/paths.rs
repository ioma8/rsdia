//! Where the captured `ASCIIFlow` fixture text files live.

use std::path::PathBuf;

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/asciiflow")
}
