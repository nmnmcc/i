//! The `i` command.

mod commands;

use clap::{Parser, Subcommand};
use ilang_cli::{BuildOptions, level};
use ilang_host::Outcome;
use ilang_package::Diagnostic;
use ilang_wasm::Target;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "i", version, about = "The i programming language")]
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
    /// write the Wasm module `build/<name>.wasm` instead.
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
        /// `build/<name>.wasm` in the package.
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
    },
    #[command(flatten)]
    Package(commands::package::PackageCommand),
    /// Inspect and trim the global cache (`$I_CACHE_DIR`, else
    /// `$XDG_CACHE_HOME/i`, else `~/.cache/i`; `I_CACHE=off` turns it off).
    #[command(subcommand)]
    Cache(commands::cache::CacheCommand),
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
                    println!("{json}");
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
        } => cmd_test(
            &project,
            opt,
            offline,
            filter.as_deref(),
            time,
            target.into(),
        ),
        Command::Package(c) => commands::package::run(c),
        Command::Cache(c) => return finish(commands::cache::run(c)),
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
    let name = if native {
        entry.unwrap_or(&b.name)
    } else {
        &b.name
    };
    let default_out = out.is_none();
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
            "the package has no program entry; define `main` in `src/main.i` or run or build a declared entry by name (§10.2)"
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
) -> anyhow::Result<ExitCode> {
    if target == Target::Wasi {
        anyhow::bail!("the WASI target runs a command entry and does not compile tests (§B.7)");
    }
    let manifest_text = std::fs::read_to_string(project.join("i.toml"))?;
    let manifest: toml_edit::DocumentMut = manifest_text.parse()?;
    let collection = manifest
        .get("test")
        .and_then(|t| t.get("collection"))
        .and_then(|v| v.as_str());
    let options = BuildOptions {
        target,
        ..options(opt, true, offline)?
    };
    let (b, entries) = match ilang_cli::build_tests(project, options, collection) {
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
    let mut seen = std::collections::HashMap::new();
    for entry in &entries {
        if let Some((_, name)) = entry.data.iter().find(|(key, _)| key == "name") {
            let qualified = format!("{}.{name}", entry.module);
            if let Some(previous) = seen.insert(qualified.clone(), &entry.at) {
                eprintln!(
                    "duplicate test name {qualified:?}: {}:{}:{} and {}:{}:{}",
                    previous.path,
                    previous.line,
                    previous.column,
                    entry.at.path,
                    entry.at.line,
                    entry.at.column
                );
                return Ok(ExitCode::FAILURE);
            }
        }
    }
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
            let name = format!(
                "{}.{}",
                entry.module,
                get("name").unwrap_or(&entry.target.path)
            );
            let mode = get("mode").unwrap_or("test");
            if filter.is_some_and(|f| !name.contains(f)) {
                continue;
            }
            if mode == "skip" || (focused && mode != "only") {
                skipped += 1;
                println!("skip {name}");
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
                println!("pass {name}{took}");
            } else {
                failed += 1;
                println!("FAIL {name}{took}");
                match outcome {
                    Ok(Outcome::Stop(stop)) => println!("  {stop}"),
                    Ok(Outcome::Err(error)) => println!("  error: {:?}", error.0),
                    Err(error) => println!("  {error:#}"),
                    Ok(Outcome::Ok(_)) => println!("  expected failure"),
                }
            }
        }
    }
    if skipped == 0 {
        println!("{passed} passed, {failed} failed");
    } else {
        println!("{passed} passed, {failed} failed, {skipped} skipped");
    }
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
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
            for plan in ilang_check::providers::resolve_program_plans(
                program,
                typed,
                &graph,
                target.provider_target(),
            )
            .into_values()
            {
                if let Err(more) = plan {
                    diags.extend(more);
                }
            }
        }
        diags.extend(checked.diags);
        if types && let (Some(program), Some(typed)) = (&checked.program, &checked.types) {
            for (i, m) in program.modules.iter().enumerate() {
                if Some(m.package) == graph.root {
                    for line in typed.lines(program, ilang_check::hir::ModuleId(i as u32)) {
                        println!("{line}");
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
