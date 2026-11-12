//! A spec file is an ordinary src/main.i, including its item macros.
use std::path::Path;

pub fn write(dir: &Path, _stem: &str, source: &str) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/main.i"), source).unwrap();
}
