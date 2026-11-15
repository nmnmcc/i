// The coordinator will move this file to /SPEC.md; change this one path then.
macro_rules! standard_path {
    () => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/spec.md")
    };
}

pub const TEXT: &str = include_str!(standard_path!());
#[cfg(test)]
#[allow(dead_code)] // Used by the integration test, not the binary test harness.
pub const PATH: &str = standard_path!();
