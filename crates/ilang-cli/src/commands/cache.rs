//! `i cache`: the global cache store (see `ilang_host::store`).
use anyhow::{Result, bail};
use ilang_host::store::{self, Limits, Store};
use std::process::ExitCode;

#[derive(clap::Subcommand)]
pub enum CacheCommand {
    /// Print where the cache is, its limits, and its entries by kind.
    Status,
    /// Remove entries unused for longer than the maximum age, then the least
    /// recently used ones until the cache is at 80% of its maximum size.
    Gc {
        /// The maximum size, in bytes or with a K, M or G suffix (default
        /// `I_CACHE_MAX_SIZE`, else 5G).
        #[arg(long, value_parser = parse_size)]
        max_size: Option<u64>,
        /// The maximum time without use, in days (default `I_CACHE_MAX_AGE`,
        /// else 14).
        #[arg(long, value_parser = parse_days)]
        max_age: Option<std::time::Duration>,
    },
    /// Remove every entry, or only the compiled code of test builds.
    Clean {
        /// Only the compiled machine code that `i test` and `i run` reuse.
        #[arg(long, conflicts_with = "all")]
        tests: bool,
        /// Everything (the default).
        #[arg(long)]
        all: bool,
    },
}

fn parse_size(text: &str) -> Result<u64, String> {
    store::parse_size(text).ok_or_else(|| format!("not a size: {text}"))
}

fn parse_days(text: &str) -> Result<std::time::Duration, String> {
    store::parse_days(text).ok_or_else(|| format!("not a number of days: {text}"))
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// The kinds of entry that `--tests` removes.
pub const TEST_KINDS: &[&str] = &[
    ilang_host::COMPILED_MODULE_KIND,
    ilang_host::COMPILED_FUNCTION_KIND,
];

pub fn run(command: CacheCommand) -> Result<ExitCode> {
    let Some(dir) = store::default_dir() else {
        bail!("no cache directory: set I_CACHE_DIR, XDG_CACHE_HOME or HOME");
    };
    let limits = Limits::from_env();
    match command {
        CacheCommand::Status => {
            let store = Store::at(&dir, limits);
            println!("directory: {}", dir.display());
            if store::disabled() {
                println!("caching: off (I_CACHE)");
            }
            println!(
                "limits: {} and {} days without use",
                human(limits.max_size),
                limits.max_age.as_secs() / (24 * 60 * 60)
            );
            let kinds = store.status();
            let (entries, bytes) = kinds
                .iter()
                .fold((0, 0), |(e, b), k| (e + k.entries, b + k.bytes));
            for kind in &kinds {
                println!(
                    "{}: {} entries, {}",
                    kind.kind,
                    kind.entries,
                    human(kind.bytes)
                );
            }
            println!("total: {entries} entries, {}", human(bytes));
        }
        CacheCommand::Gc { max_size, max_age } => {
            let store = Store::at(
                &dir,
                Limits {
                    max_size: max_size.unwrap_or(limits.max_size),
                    max_age: max_age.unwrap_or(limits.max_age),
                },
            );
            let Some(collected) = store.gc() else {
                eprintln!("another process is collecting the cache");
                return Ok(ExitCode::FAILURE);
            };
            println!(
                "removed {} entries ({}); kept {} entries ({})",
                collected.removed_entries,
                human(collected.removed_bytes),
                collected.kept_entries,
                human(collected.kept_bytes)
            );
        }
        CacheCommand::Clean { tests, all: _ } => {
            let store = Store::at(&dir, limits);
            let Some(collected) = store.clean(tests.then_some(TEST_KINDS)) else {
                bail!("cannot lock the cache directory {}", dir.display());
            };
            println!(
                "removed {} entries ({})",
                collected.removed_entries,
                human(collected.removed_bytes)
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}
