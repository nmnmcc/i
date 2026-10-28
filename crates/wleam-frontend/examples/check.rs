//! Check a project using only the compiler front end.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let root = arguments.next().unwrap_or_else(|| ".".into());
    let options = wleam_frontend::CheckOptions {
        include_tests: arguments.any(|argument| argument == "--tests"),
        ..Default::default()
    };
    let checked = wleam_frontend::check_project(root, &options)?;
    for warning in checked.warnings {
        eprintln!("{warning}");
    }
    println!(
        "Checked {} modules in {}: {}",
        checked.modules.len(),
        checked.package_name,
        serde_json::to_string(&checked.stats)?,
    );
    Ok(())
}
