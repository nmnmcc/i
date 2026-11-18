//! The `i` command.

mod commands;
mod output;

use clap::{Parser, Subcommand};
use ilang_cli::{BuildOptions, level};
use ilang_host::Outcome;
use ilang_package::Diagnostic;
use ilang_wasm::Target;
use output::outln;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "i", version, about = "The i Programming Language")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print source after macro expansion, without running the program.
    Expand {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        offline: bool,
        /// Output surface; quotations require is.
        #[arg(long, default_value = "is", value_parser = ["i", "is"])]
        to: String,
        /// Print each macro's arguments and result on standard error.
        #[arg(long)]
        trace: bool,
        /// A source file or qualified module; by default all package modules.
        module: Option<String>,
    },
    /// Check a package for errors.
    Check {
        /// The package directory, with its `i.toml`.
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Do not use the network to get packages; only the cache is used.
        #[arg(long)]
        offline: bool,
        /// Print the type of every top-level name of the package, as
        /// `module/path.name : type`.
        #[arg(long)]
        types: bool,
        /// Validate entry provider plans for this target.
        #[arg(long, value_enum)]
        target: Option<TargetArg>,
    },
    /// Build a package into a native executable, or a Wasm module.
    ///
    /// The default target, native, makes one executable file,
    /// `build/<name>` (the package name, or the entry NAME), which runs
    /// without `i`: a runner with Wasmtime's runtime and the standard host
    /// services, and the program compiled ahead of time. On Linux it is
    /// statically linked (musl). It is built for the operating system and
    /// architecture of this machine only, with a baseline of CPU features
    /// (x86-64-v2 on x86-64); building for other targets and signing are not
    /// supported yet. `--target host` (or `ilang`), `js`, `nodejs` and `wasi`
    /// write the Wasm module `build/<name>.wasm` instead, with the same
    /// package name or entry NAME.
    Build {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Do not use the network to get packages; only the cache is used.
        #[arg(long)]
        offline: bool,
        /// Optimization level: 0, 2, or 3.
        #[arg(long, default_value_t = 2)]
        opt: u8,
        /// Select a function declared in [entries].
        entry: Option<String>,
        /// The output file; by default `build/<name>` (native) or
        /// `build/<name>.wasm` in the package, named after the package or
        /// the selected entry.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Provider column; ilang is an alias of host.
        #[arg(long, value_enum, default_value_t = TargetArg::Native)]
        target: TargetArg,
        /// Also print the unoptimized WAT text next to the output.
        #[arg(long, hide = true)]
        wat: bool,
    },
    /// Build and run the program entry of a package.
    Run {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Do not use the network to get packages; only the cache is used.
        #[arg(long)]
        offline: bool,
        /// Optimization level: 0, 2, or 3.
        #[arg(long, default_value_t = 2)]
        opt: u8,
        #[arg(long, value_enum, default_value_t = TargetArg::Native)]
        target: TargetArg,
        /// Select a function declared in [entries].
        entry: Option<String>,
        /// Arguments passed to the program after --.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// List checked collection contributions as a JSON manifest.
    Collections {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        offline: bool,
        /// Include test modules and development dependencies.
        #[arg(long)]
        tests: bool,
    },
    /// Build and run one opaque collection handle.
    RunCollection {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        tests: bool,
        #[arg(long, default_value_t = 2)]
        opt: u8,
        /// Require the selected function to give Nil.
        #[arg(long)]
        require_nil: bool,
        handle: String,
    },
    /// Build and run the tests of a package.
    Test {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Do not use the network to get packages; only the cache is used.
        #[arg(long)]
        offline: bool,
        /// Optimization level: 0, 2, or 3.
        #[arg(long, default_value_t = 0)]
        opt: u8,
        #[arg(long, value_enum, default_value_t = TargetArg::Native)]
        target: TargetArg,
        /// Only run the tests whose name has this text.
        filter: Option<String>,
        /// Print how long each test takes to run, without the build.
        #[arg(long)]
        time: bool,
        /// Run only shard K of N (0 <= K < N), planned from measured costs;
        /// the name filter applies within the shard.
        #[arg(long, value_name = "K/N")]
        shard: Option<String>,
    },
    #[command(flatten)]
    Package(commands::package::PackageCommand),
    /// Inspect and trim the global cache (`$I_CACHE_DIR`, else
    /// `$XDG_CACHE_HOME/i`, else `~/.cache/i`; `I_CACHE=off` turns it off).
    #[command(subcommand)]
    Cache(commands::cache::CacheCommand),
    /// Read the language standard.
    Spec(commands::spec::SpecArgs),
    /// Run the language server, on standard input and output.
    Lsp,
    /// Format source files; by default every `.i` and `.is` file in `src/` and `test/`.
    Fmt {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Write nothing; list the files that would change, and fail if any.
        #[arg(long)]
        check: bool,
        /// Convert the source to the specified surface.
        #[arg(long, value_parser = ["i", "is"])]
        to: Option<String>,
        /// Files or directories to format instead.
        paths: Vec<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Expand {
            project,
            offline,
            to,
            trace,
            module,
        } => commands::expand::run(&project, offline, &to, trace, module.as_deref()),
        Command::Check {
            project,
            offline,
            types,
            target,
        } => {
            let code = check(&project, offline, types, target.map(Into::into));
            ilang_host::store::maintain_global();
            return code;
        }
        Command::Build {
            project,
            offline,
            opt,
            out,
            target,
            wat,
            entry,
        } => cmd_build(
            &project,
            opt,
            offline,
            out,
            target.into(),
            wat,
            entry.as_deref(),
        ),
        Command::Run {
            project,
            offline,
            opt,
            entry,
            args,
            target,
        } => cmd_run(
            &project,
            opt,
            offline,
            entry.as_deref(),
            args,
            target.into(),
        ),
        Command::Collections {
            project,
            offline,
            tests,
        } => match ilang_cli::collections(&project, tests, offline) {
            Ok(manifest) => match serde_json::to_string_pretty(&manifest) {
                Ok(json) => {
                    outln!("{json}");
                    Ok(ExitCode::SUCCESS)
                }
                Err(error) => Err(error.into()),
            },
            Err(error) => {
                eprintln!("{error}");
                Ok(ExitCode::FAILURE)
            }
        },
        Command::RunCollection {
            project,
            offline,
            tests,
            opt,
            require_nil,
            handle,
        } => cmd_run_collection(&project, opt, offline, tests, &handle, require_nil),
        Command::Test {
            project,
            offline,
            opt,
            filter,
            time,
            target,
            shard,
        } => cmd_test(
            &project,
            opt,
            offline,
            filter.as_deref(),
            time,
            target.into(),
            shard.as_deref(),
        ),
        Command::Package(c) => commands::package::run(c),
        Command::Cache(c) => return finish(commands::cache::run(c)),
        Command::Spec(args) => return finish(commands::spec::run(args)),
        Command::Lsp => commands::lsp::run(),
        Command::Fmt {
            project,
            check,
            to,
            paths,
        } => commands::fmt::run(&project, check, to.as_deref(), &paths),
    };
    // Collect the cache after the work, when it is due (at most once a day,
    // or at once when this run pushed it past 120% of its cap).
    ilang_host::store::maintain_global();
    finish(result)
}

fn finish(result: anyhow::Result<ExitCode>) -> ExitCode {
    match result {
        Ok(code) => code,
        // The reader of standard output has gone, as in `i spec | head`; the
        // output was cut short on purpose, so this is not a failure.
        Err(e) if output::is_broken_pipe(&e) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

/// The values of `--target`.
#[derive(Clone, Copy, clap::ValueEnum)]
pub(crate) enum TargetArg {
    Js,
    Nodejs,
    Native,
    Wasi,
    #[value(alias = "ilang")]
    Host,
}

impl From<TargetArg> for Target {
    fn from(t: TargetArg) -> Target {
        match t {
            TargetArg::Js => Target::Js,
            TargetArg::Nodejs => Target::Nodejs,
            TargetArg::Native => Target::Native,
            TargetArg::Wasi => Target::Wasi,
            TargetArg::Host => Target::Host,
        }
    }
}

fn options(opt: u8, tests: bool, offline: bool) -> anyhow::Result<BuildOptions> {
    let opt = level(opt).ok_or_else(|| anyhow::anyhow!("--opt must be 0, 2, or 3"))?;
    Ok(BuildOptions {
        tests,
        opt,
        offline,
        target: Target::Native,
    })
}

fn built_entry(
    project: &Path,
    options: BuildOptions,
    entry: Option<&str>,
) -> anyhow::Result<Option<ilang_cli::Built>> {
    match ilang_cli::build_entry(project, options, entry) {
        Ok(b) => {
            for w in &b.warnings {
                eprintln!("{w}");
            }
            Ok(Some(b))
        }
        Err(e @ ilang_cli::BuildError::Program { .. }) => {
            eprintln!("{e}");
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

fn cmd_build(
    project: &Path,
    opt: u8,
    offline: bool,
    out: Option<PathBuf>,
    target: Target,
    wat: bool,
    entry: Option<&str>,
) -> anyhow::Result<ExitCode> {
    let options = BuildOptions {
        target,
        ..options(opt, false, offline)?
    };
    let Some(b) = built_entry(project, options, entry)? else {
        return Ok(ExitCode::FAILURE);
    };
    let native = target == Target::Native;
    // Name the artifact after the selected entry, as native packages do, so
    // that entries (and the package default) do not overwrite each other.
    let name = entry.unwrap_or(&b.name);
    let default_out = out.is_none();
    if default_out && entry == Some(b.name.as_str()) {
        // `build/<name>` of this entry is also the default entry's artifact;
        // writing it would silently replace the other build.
        eprintln!(
            "error: the entry `{name}` has the package's name, so its default artifact would replace the default entry's; choose the path with --out"
        );
        return Ok(ExitCode::FAILURE);
    }
    let out = out.unwrap_or_else(|| {
        project.join("build").join(if native {
            name.to_owned()
        } else {
            format!("{name}.wasm")
        })
    });
    let dir = out
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    if default_out {
        // The package's `build/` is generated output: mark it for backup tools.
        ilang_host::store::write_cachedir_tag(dir);
    }
    if native {
        let package = match b.native_package(name) {
            Ok(package) => package,
            Err(error) => {
                eprintln!("error: {error:#}");
                return Ok(ExitCode::FAILURE);
            }
        };
        // Write a new file and rename it, so that a running copy of the
        // old executable is not changed under it.
        let mut file = tempfile::NamedTempFile::new_in(dir)?;
        std::io::Write::write_all(&mut file, &package)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o755))?;
        }
        file.persist(&out)?;
    } else {
        std::fs::write(&out, &b.wasm)?;
        if target == ilang_wasm::Target::Wasi && default_out {
            // A WASI module has no binding plan. A plan left by an earlier
            // non-WASI build of the same name in the generated `build/` would
            // describe a module that is no longer there.
            match std::fs::remove_file(out.with_extension("bindings.json")) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    if !native && target != ilang_wasm::Target::Wasi {
        std::fs::write(
            out.with_extension("bindings.json"),
            b.binding_plan.to_json()?,
        )?;
    }
    if wat {
        let (mut graph, _) = ilang_cli::load(project, false, offline);
        graph.entry = entry.map(str::to_owned);
        let checked = ilang_macro::project::check(&mut graph);
        if let (Some(p), Some(t)) = (&checked.program, &checked.types)
            && let Ok(text) = ilang_wasm::compile_wat_numeric(
                &graph,
                p,
                t,
                ilang_wasm::Options {
                    tests: false,
                    target,
                }
                .numeric(if opt == 0 {
                    ilang_wasm::Numeric::Reference
                } else {
                    ilang_wasm::Numeric::Scalars
                }),
            )
        {
            std::fs::write(out.with_extension("wat"), text)?;
        }
    }
    eprintln!("built {}", out.display());
    Ok(ExitCode::SUCCESS)
}

fn cmd_run(
    project: &Path,
    opt: u8,
    offline: bool,
    entry: Option<&str>,
    args: Vec<String>,
    target: Target,
) -> anyhow::Result<ExitCode> {
    let options = BuildOptions {
        target,
        ..options(opt, false, offline)?
    };
    let Some(b) = built_entry(project, options, entry)? else {
        return Ok(ExitCode::FAILURE);
    };
    if target == Target::Wasi {
        let file = tempfile::Builder::new().suffix(".wasm").tempfile()?;
        std::fs::write(file.path(), &b.wasm)?;
        let status = std::process::Command::new(
            std::env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into()),
        )
        .arg("run")
        .arg(file.path())
        .args(args)
        .status()?;
        return Ok(status
            .code()
            .map_or(ExitCode::FAILURE, |code| ExitCode::from(code as u8)));
    }
    let program = b.host_program()?;
    if program.abi().entry.is_none() {
        anyhow::bail!(
            "the package has no program entry; define `main` in `src/main.i` or run or build a declared entry by name"
        );
    }
    let mut options = ilang_host::HostOptions::new(ilang_host::services::stdout_with_args(args));
    options
        .imports
        .grant_package(&b.binding_plan, &b.trusted_package);
    match program.run_entry(options)? {
        Outcome::Ok(_) => Ok(ExitCode::SUCCESS),
        Outcome::Stop(stop) => {
            eprintln!("{stop}");
            Ok(ExitCode::FAILURE)
        }
        Outcome::Err(error) => {
            eprintln!("error: {:?}", error.0);
            Ok(ExitCode::FAILURE)
        }
    }
}

fn cmd_run_collection(
    project: &Path,
    opt: u8,
    offline: bool,
    tests: bool,
    handle: &str,
    require_nil: bool,
) -> anyhow::Result<ExitCode> {
    let result = if require_nil {
        ilang_cli::CollectionResult::Nil
    } else {
        ilang_cli::CollectionResult::Any
    };
    let selected =
        match ilang_cli::build_collection(project, options(opt, tests, offline)?, handle, result) {
            Ok(selected) => selected,
            Err(error) => {
                eprintln!("{error}");
                return Ok(ExitCode::FAILURE);
            }
        };
    for warning in &selected.built.warnings {
        eprintln!("{warning}");
    }
    match selected.run_with(selected.built.host_options())? {
        Outcome::Ok(_) => Ok(ExitCode::SUCCESS),
        Outcome::Stop(stop) => {
            eprintln!("{stop}");
            Ok(ExitCode::FAILURE)
        }
        Outcome::Err(error) => {
            eprintln!("error: {:?}", error.0);
            Ok(ExitCode::FAILURE)
        }
    }
}

fn cmd_test(
    project: &Path,
    opt: u8,
    offline: bool,
    filter: Option<&str>,
    time: bool,
    target: Target,
    shard: Option<&str>,
) -> anyhow::Result<ExitCode> {
    let shard = match shard.map(parse_shard).transpose() {
        Ok(shard) => shard,
        Err(message) => {
            eprintln!("error: {message}");
            return Ok(ExitCode::FAILURE);
        }
    };
    if target == Target::Wasi {
        anyhow::bail!("the WASI target runs a command entry and does not compile tests");
    }
    let manifest_text = std::fs::read_to_string(project.join("i.toml"))?;
    let manifest: toml_edit::DocumentMut = manifest_text.parse()?;
    let collection = match ilang_cli::test_collection(&manifest) {
        Ok(collection) => collection,
        Err(message) => {
            eprintln!("error: {message}");
            return Ok(ExitCode::FAILURE);
        }
    };
    let options = BuildOptions {
        target,
        ..options(opt, true, offline)?
    };
    let (b, entries) = match ilang_cli::build_tests(project, options, collection.as_deref()) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return Ok(ExitCode::FAILURE);
        }
    };
    for warning in &b.warnings {
        eprintln!("{warning}");
    }
    let focused = entries
        .iter()
        .any(|e| e.data.iter().any(|(k, v)| k == "mode" && v == "only"));
    if let Some(message) = ilang_cli::duplicate_test_name(&entries) {
        eprintln!("{message}");
        return Ok(ExitCode::FAILURE);
    }
    let entries = match shard {
        None => entries,
        Some(shard) => match shard_entries(entries, shard) {
            Ok(entries) => entries,
            Err(message) => {
                eprintln!("error: {message}");
                return Ok(ExitCode::FAILURE);
            }
        },
    };
    let stage = ilang_syntax::trace::stage("Wasmtime compile");
    let program = b.host_program()?;
    drop(stage);
    let mut failed = 0;
    let mut passed = 0;
    let mut skipped = 0;
    {
        let _stage = ilang_syntax::trace::stage("run tests");
        for entry in entries {
            let get = |key: &str| {
                entry
                    .data
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.as_str())
            };
            let name = test_name(&entry);
            let mode = get("mode").unwrap_or("test");
            if filter.is_some_and(|f| !name.contains(f)) {
                continue;
            }
            if mode == "skip" || (focused && mode != "only") {
                skipped += 1;
                outln!("skip {name}");
                continue;
            }
            let index = entry
                .handle
                .rsplit(':')
                .next()
                .expect("collection handle index");
            let export = format!("i.tool.{index}");
            let start = (std::time::Instant::now(), cpu_seconds());
            let outcome = ilang_host::block_on(async {
                let mut instance = program.instantiate(b.host_options())?;
                if program
                    .abi()
                    .tests
                    .iter()
                    .any(|t| t.name == export && t.is_async)
                {
                    instance.run_async(&export).await
                } else {
                    instance.run(&export)
                }
            });
            let took = if time {
                let wall = start.0.elapsed().as_secs_f64();
                match (start.1, cpu_seconds()) {
                    (Some(a), Some(b)) => format!(" ({wall:.3} s, cpu {:.2} s)", b - a),
                    _ => format!(" ({wall:.3} s)"),
                }
            } else {
                String::new()
            };
            let success = matches!(outcome, Ok(Outcome::Ok(_)));
            if outcome.is_ok() && success != (mode == "fails") {
                passed += 1;
                outln!("pass {name}{took}");
            } else {
                failed += 1;
                outln!("FAIL {name}{took}");
                match outcome {
                    Ok(Outcome::Stop(stop)) => outln!("  {stop}"),
                    Ok(Outcome::Err(error)) => outln!("  error: {:?}", error.0),
                    Err(error) => outln!("  {error:#}"),
                    Ok(Outcome::Ok(_)) => outln!("  expected failure"),
                }
            }
        }
    }
    if skipped == 0 {
        outln!("{passed} passed, {failed} failed");
    } else {
        outln!("{passed} passed, {failed} failed, {skipped} skipped");
    }
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The qualified name of a test entry: `module.name`.
fn test_name(entry: &ilang_cli::CollectionEntry) -> String {
    let name = entry
        .data
        .iter()
        .find(|(k, _)| k == "name")
        .map(|(_, v)| v.as_str())
        .unwrap_or(&entry.target.path);
    format!("{}.{name}", entry.module)
}

/// `K/N` of `--shard`: N positive and K below it.
fn parse_shard(text: &str) -> Result<(usize, usize), String> {
    let parsed = text
        .split_once('/')
        .and_then(|(k, n)| Some((k.parse::<usize>().ok()?, n.parse::<usize>().ok()?)));
    match parsed {
        Some((k, n)) if n > 0 && k < n => Ok((k, n)),
        Some((k, n)) if n > 0 => Err(format!(
            "--shard {text}: shard {k} is out of range; there are {n} shards, numbered 0 to {}",
            n - 1
        )),
        Some(_) => Err(format!("--shard {text}: the number of shards must be positive")),
        None => Err(format!("--shard {text}: expected K/N, two whole numbers")),
    }
}

/// The shard budget when the cost table gives none.
const SHARD_BUDGET_SECONDS: f64 = 270.0;
/// The set of `ci/costs.json` that holds the costs of tests run by `i test`.
const SHARD_COST_SET: &str = "ilang-cli-tests";

/// The entries of shard `k` of `n`, sorted by qualified name. The shards are
/// planned by `ilang_shard::plan` from the costs in `ci/costs.json` of the
/// workspace (or `CHECKS_COSTS`); without a table every test costs the same.
fn shard_entries(
    entries: Vec<ilang_cli::CollectionEntry>,
    (k, n): (usize, usize),
) -> Result<Vec<ilang_cli::CollectionEntry>, String> {
    let mut named: Vec<(String, ilang_cli::CollectionEntry)> =
        entries.into_iter().map(|e| (test_name(&e), e)).collect();
    named.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some(pair) = named.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(format!(
            "--shard {k}/{n}: two tests are named {:?}; a shard needs every name once",
            pair[0].0
        ));
    }
    let path = std::env::var_os("CHECKS_COSTS").map(PathBuf::from).unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ci/costs.json")
    });
    let table = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|text| ilang_shard::CostTable::parse(&text));
    let (units, budget) = match table {
        Ok(table) if table.sets.contains_key(SHARD_COST_SET) => {
            let set = &table.sets[SHARD_COST_SET];
            // The tests first, then the other keys of the set, so that a
            // test without a cost takes the median of the whole set.
            let mut pairs: Vec<(String, Option<f64>)> = named
                .iter()
                .map(|(name, _)| (name.clone(), set.costs.get(name).copied()))
                .collect();
            let names: std::collections::BTreeSet<&str> =
                named.iter().map(|(name, _)| name.as_str()).collect();
            pairs.extend(
                set.keys
                    .iter()
                    .filter(|key| !names.contains(key.as_str()))
                    .map(|key| (key.clone(), set.costs.get(key).copied())),
            );
            let mut units = ilang_shard::with_median_missing(&pairs);
            units.truncate(named.len());
            (units, table.budget_seconds.unwrap_or(SHARD_BUDGET_SECONDS))
        }
        other => {
            let reason = match other {
                Ok(_) => format!("it has no set {SHARD_COST_SET:?}"),
                Err(error) => error,
            };
            eprintln!(
                "note: --shard: every test costs the same; no cost table at {} ({reason})",
                path.display()
            );
            let units = named
                .iter()
                .map(|(name, _)| ilang_shard::Unit { key: name.clone(), cost: 1.0 })
                .collect();
            (units, SHARD_BUDGET_SECONDS)
        }
    };
    let plan = ilang_shard::plan(&units, budget);
    let count = plan.shards.len();
    if count != n {
        return Err(format!(
            "--shard {k}/{n}: the {} tests make {count} shards, not {n}",
            named.len()
        ));
    }
    let keys: std::collections::BTreeSet<&String> = plan.shards[k].keys.iter().collect();
    Ok(named
        .into_iter()
        .filter(|(name, _)| keys.contains(name))
        .map(|(_, entry)| entry)
        .collect())
}

/// The processor time of the process so far (user and system), where the
/// system tells it: on a busy machine it is steadier than the time taken.
fn cpu_seconds() -> Option<f64> {
    // Fields 14 and 15 of /proc/self/stat, after the command name in
    // parentheses, in clock ticks of 1/100 s.
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let rest = &stat[stat.rfind(')')? + 2..];
    let mut fields = rest.split(' ').skip(11);
    let user: f64 = fields.next()?.parse().ok()?;
    let system: f64 = fields.next()?.parse().ok()?;
    Some((user + system) / 100.0)
}

/// Parses, resolves, and type checks every module of the package, its
/// dependencies, and the library, and prints the problems as
/// `path:line:col: error[CODE]: …`. With `types`, prints the type of each
/// top-level name of the package's modules, in source order. The tests are
/// checked too.
fn check(project: &Path, offline: bool, types: bool, target: Option<Target>) -> ExitCode {
    let (mut graph, mut diags) = ilang_cli::load(project, true, offline);
    if graph.root.is_some() {
        let checked = ilang_macro::project::check(&mut graph);
        if let Some(target) = target
            && !checked.diags.iter().any(Diagnostic::is_error)
            && let (Some(program), Some(typed)) = (&checked.program, &checked.types)
        {
            for stub in
                ilang_check::platform::stubs(program, typed, target.platform()).into_values()
            {
                if let Err(more) = stub {
                    diags.extend(more);
                }
            }
        }
        diags.extend(checked.diags);
        if types && let (Some(program), Some(typed)) = (&checked.program, &checked.types) {
            for (i, m) in program.modules.iter().enumerate() {
                if Some(m.package) == graph.root {
                    for line in typed.lines(program, ilang_check::hir::ModuleId(i as u32)) {
                        outln!("{line}");
                    }
                }
            }
        }
    }
    for line in ilang_cli::diagnostic_lines(&graph, &diags) {
        eprintln!("{line}");
    }
    let errors = diags.iter().filter(|d| d.is_error()).count();
    if errors > 0 || graph.root.is_none() {
        eprintln!("{errors} error{}", if errors == 1 { "" } else { "s" });
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
