//! Optional named entry declared by a spec case's header.
use std::path::Path;

pub fn named_entry(project: &Path, tests: bool) -> Option<String> {
    std::fs::read_to_string(project.join("i.toml"))
        .expect("spec manifest")
        .lines()
        .find_map(|line| {
            line.strip_prefix("# entry:")
                .or_else(|| tests.then(|| line.strip_prefix("# entry-test:")).flatten())
                .map(|name| name.trim().to_string())
        })
}
