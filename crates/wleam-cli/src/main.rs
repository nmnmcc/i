//! Wleam's project-oriented command line interface.
use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::Instant,
};
use wleam_frontend::{CacheStats, CheckOptions, CheckedProject, check_project};

#[derive(Debug, Parser)]
#[command(name = "wleam", version, about = "Gleam for WebAssembly GC")]
struct Cli {
    /// Project directory containing gleam.toml.
    #[arg(long, global = true, default_value = ".")]
    project: PathBuf,
    /// Report command metadata as JSON.
    #[arg(long, global = true)]
    json: bool,
    /// Recompute frontend, IR, and output instead of loading cached results.
    #[arg(long, global = true)]
    no_cache: bool,
    #[command(subcommand)]
    command: Action,
}

#[derive(Debug, Subcommand)]
enum Action {
    /// Type-check the application and its local dependencies.
    Check {
        #[arg(long)]
        tests: bool,
    },
    /// Compile the project to a validated WasmGC module.
    Build {
        /// Compile configured library exports without a main entry point.
        #[arg(long)]
        lib: bool,
        /// Override the default package.main entry point.
        #[arg(long)]
        entry: Option<String>,
        /// Write the Wasm module to this path.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Build and execute the application's main function with Wasmtime.
    Run {
        #[arg(long)]
        entry: Option<String>,
        #[arg(long)]
        runner: Option<PathBuf>,
    },
    /// Discover and execute public, zero-argument *_test functions.
    Test {
        /// Only run tests whose qualified name contains this text.
        filter: Option<String>,
        #[arg(long)]
        runner: Option<PathBuf>,
    },
    /// Format Gleam files in src and test, or explicitly supplied paths.
    Fmt {
        /// Fail if formatting would change any file.
        #[arg(long)]
        check: bool,
        paths: Vec<PathBuf>,
    },
    /// Start the language server using the Language Server Protocol over stdio.
    Lsp,
}

#[derive(Serialize)]
struct Report<'a> {
    command: &'a str,
    package: &'a str,
    modules: usize,
    elapsed_ms: u128,
    cache: &'a CacheStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_cache_hit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lowering: Option<&'a wleam_wasm::lower::CacheStats>,
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn execute(cli: Cli) -> Result<u8> {
    let started = Instant::now();
    match cli.command {
        Action::Lsp => {
            wleam_lsp::run()?;
            Ok(0)
        }
        Action::Fmt { check, paths } => {
            format_files(&cli.project, &paths, check, cli.json)?;
            Ok(0)
        }
        Action::Check { tests } => {
            let project = checked(&cli.project, cli.no_cache, tests)?;
            report(&project, "check", None, started, cli.json)?;
            Ok(0)
        }
        Action::Build { lib, entry, out } => {
            let project = checked(&cli.project, cli.no_cache, false)?;
            let path = build(&project, lib, entry, &[], out.as_deref(), !cli.no_cache)?;
            report(&project, "build", Some(&path), started, cli.json)?;
            Ok(0)
        }
        Action::Run { entry, runner } => {
            let project = checked(&cli.project, cli.no_cache, false)?;
            let path = build(&project, false, entry, &[], None, !cli.no_cache)?;
            invoke_runner(runner.as_deref(), &path.path, &[])
        }
        Action::Test { filter, runner } => {
            let project = checked(&cli.project, cli.no_cache, true)?;
            let tests = discover_tests(&project, filter.as_deref())?;
            if tests.is_empty() {
                bail!("no matching public zero-argument *_test functions were found in test/");
            }
            let path = project
                .root
                .join("build/wasm")
                .join(format!("{}_test.wasm", project.package_name));
            let path = build(&project, true, None, &tests, Some(&path), !cli.no_cache)?;
            let mut args = vec![OsString::from("--all-tests")];
            if cli.json {
                args.push(OsString::from("--json"));
            }
            invoke_runner(runner.as_deref(), &path.path, &args)
        }
    }
}

fn checked(root: &Path, no_cache: bool, include_tests: bool) -> Result<CheckedProject> {
    let options = CheckOptions {
        include_tests,
        use_cache: !no_cache,
        ..CheckOptions::default()
    };
    check_project(root, &options).map_err(|error| anyhow!("{error}"))
}

fn build(
    project: &CheckedProject,
    library: bool,
    entry: Option<String>,
    tests: &[String],
    out: Option<&Path>,
    use_cache: bool,
) -> Result<BuildOutput> {
    if !library && entry.is_none() {
        let has_main_module = project
            .sources
            .get(&project.package_name)
            .is_some_and(|source| source.package_name == project.package_name);
        if !has_main_module {
            bail!(
                "application package {} must define src/{}.gleam with a zero-argument main; use --entry to select another module",
                project.package_name,
                project.package_name
            );
        }
    }
    let options = wleam_wasm::Options {
        entry: entry.unwrap_or_else(|| format!("{}.main", project.package_name)),
        exports: project.exports.clone(),
        library,
        tests: tests.to_vec(),
    };
    let output = out.map(Path::to_path_buf).unwrap_or_else(|| {
        project
            .root
            .join("build/wasm")
            .join(format!("{}.wasm", project.package_name))
    });
    if output
        .extension()
        .is_none_or(|extension| extension != "wasm")
    {
        bail!(
            "output path must end in .wasm so ABI and JavaScript companion files have distinct paths"
        );
    }
    let fingerprint = artifact_fingerprint(project, &options);
    if use_cache && artifact_is_current(&output, &fingerprint) {
        return Ok(BuildOutput {
            path: output,
            cache_hit: true,
            lowering: None,
        });
    }
    let (artifact, lowering) = if use_cache {
        let (artifact, stats) = wleam_wasm::compile_cached(
            &project.modules,
            &options,
            &project.root.join(".wleam/ir"),
            wleam_frontend::compiler_fingerprint(),
        )?;
        (artifact, Some(stats))
    } else {
        (wleam_wasm::compile(&project.modules, &options)?, None)
    };
    write_atomic(&output, &artifact.wasm)?;
    write_atomic(
        &output.with_extension("abi.json"),
        &serde_json::to_vec_pretty(&artifact.abi)?,
    )?;
    write_atomic(
        &output.with_extension("mjs"),
        include_bytes!("../../../bindings/javascript/wleam.mjs"),
    )?;
    write_atomic(
        &output.with_extension("d.ts"),
        include_bytes!("../../../bindings/javascript/wleam.d.ts"),
    )?;
    let record = ArtifactCache {
        fingerprint,
        wasm_sha256: digest(&artifact.wasm),
        abi_sha256: digest(&fs::read(output.with_extension("abi.json"))?),
    };
    write_atomic(
        &output.with_extension("build.json"),
        &serde_json::to_vec(&record)?,
    )?;
    Ok(BuildOutput {
        path: output,
        cache_hit: false,
        lowering,
    })
}

struct BuildOutput {
    path: PathBuf,
    cache_hit: bool,
    lowering: Option<wleam_wasm::lower::CacheStats>,
}

#[derive(Serialize, Deserialize)]
struct ArtifactCache {
    fingerprint: String,
    wasm_sha256: String,
    abi_sha256: String,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn artifact_fingerprint(project: &CheckedProject, options: &wleam_wasm::Options) -> String {
    let mut hash = Sha256::new();
    hash.update(b"wleam-artifact-cache-v1\0");
    hash.update(wleam_frontend::compiler_fingerprint().as_bytes());
    for (name, source) in &project.sources {
        for bytes in [
            name.as_bytes(),
            source.package_name.as_bytes(),
            source.code.as_bytes(),
        ] {
            hash.update((bytes.len() as u64).to_le_bytes());
            hash.update(bytes);
        }
    }
    hash.update(
        serde_json::json!({
            "entry":options.entry, "exports":options.exports,
            "library":options.library, "tests":options.tests,
        })
        .to_string()
        .as_bytes(),
    );
    format!("{hash:x}", hash = hash.finalize())
}

fn artifact_is_current(output: &Path, fingerprint: &str) -> bool {
    let current = || -> Option<bool> {
        let record: ArtifactCache =
            serde_json::from_slice(&fs::read(output.with_extension("build.json")).ok()?).ok()?;
        Some(
            record.fingerprint == fingerprint
                && record.wasm_sha256 == digest(&fs::read(output).ok()?)
                && record.abi_sha256 == digest(&fs::read(output.with_extension("abi.json")).ok()?)
                && fs::read(output.with_extension("mjs")).ok()?.as_slice()
                    == include_bytes!("../../../bindings/javascript/wleam.mjs")
                && fs::read(output.with_extension("d.ts")).ok()?.as_slice()
                    == include_bytes!("../../../bindings/javascript/wleam.d.ts"),
        )
    };
    current() == Some(true)
}

fn report(
    project: &CheckedProject,
    command: &str,
    artifact: Option<&BuildOutput>,
    started: Instant,
    json: bool,
) -> Result<()> {
    let elapsed = started.elapsed();
    if json {
        let report = Report {
            command,
            package: &project.package_name,
            modules: project.modules.len(),
            elapsed_ms: elapsed.as_millis(),
            cache: &project.stats,
            artifact: artifact.map(|output| output.path.display().to_string()),
            artifact_cache_hit: artifact.map(|output| output.cache_hit),
            lowering: artifact.and_then(|output| output.lowering.as_ref()),
        };
        println!("{}", serde_json::to_string(&report)?);
    } else if let Some(artifact) = artifact {
        eprintln!(
            "Built {} → {} ({:.2}s)",
            project.package_name,
            artifact.path.display(),
            elapsed.as_secs_f64()
        );
    } else {
        eprintln!(
            "Checked {} ({} modules, {:.2}s)",
            project.package_name,
            project.modules.len(),
            elapsed.as_secs_f64()
        );
    }
    Ok(())
}

fn discover_tests(project: &CheckedProject, filter: Option<&str>) -> Result<Vec<String>> {
    let mut tests = Vec::new();
    for module in &project.modules {
        let Some(source) = project.sources.get(module.name.as_str()) else {
            continue;
        };
        if !source.is_test || source.package_name != project.package_name {
            continue;
        }
        for function in &module.definitions.functions {
            let Some((_, name)) = &function.name else {
                continue;
            };
            if !function.publicity.is_importable() || !name.ends_with("_test") {
                continue;
            }
            let qualified = format!("{}.{}", module.name, name);
            if !function.arguments.is_empty() || !function.return_type.is_nil() {
                bail!("test {qualified} must accept no arguments and return Nil");
            }
            if filter.is_none_or(|filter| qualified.contains(filter)) {
                tests.push(qualified);
            }
        }
    }
    tests.sort();
    Ok(tests)
}

fn runner_command(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    if let Some(path) = std::env::var_os("WLEAM_RUNNER") {
        return Ok(path.into());
    }
    if let Some(parent) = std::env::current_exe()?.parent() {
        let sibling = parent.join(format!("wleam-runner{}", std::env::consts::EXE_SUFFIX));
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    Ok(PathBuf::from("wleam-runner"))
}

fn invoke_runner(explicit: Option<&Path>, module: &Path, args: &[OsString]) -> Result<u8> {
    let runner = runner_command(explicit)?;
    let status = Command::new(&runner)
        .arg(module)
        .args(args)
        .status()
        .with_context(|| {
            format!(
                "could not start {}; build/install wleam-runner or set WLEAM_RUNNER",
                runner.display()
            )
        })?;
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1))
}

fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let name = path
        .file_name()
        .context("output path must name a file")?
        .to_string_lossy();
    let temporary = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.with_context(|| format!("write {}", path.display()))
}

fn gleam_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path
            .extension()
            .is_some_and(|extension| extension == "gleam")
        {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if !entry.file_type()?.is_symlink() {
                gleam_files(&entry.path(), files)?;
            }
        }
    }
    Ok(())
}

fn format_files(root: &Path, paths: &[PathBuf], check: bool, json: bool) -> Result<()> {
    let mut files = Vec::new();
    if paths.is_empty() {
        gleam_files(&root.join("src"), &mut files)?;
        gleam_files(&root.join("test"), &mut files)?;
    } else {
        for path in paths {
            if !path.exists() {
                bail!("format input does not exist: {}", path.display());
            }
            gleam_files(path, &mut files)?;
        }
    }
    files.sort();
    files.dedup();
    let mut changed = Vec::new();
    // Parse all files before making changes, so a syntax error cannot leave a partially formatted project.
    let mut edits = Vec::new();
    for path in &files {
        let source =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let formatted =
            wleam_frontend::format_source(path, &source).map_err(|error| anyhow!("{error}"))?;
        if formatted != source {
            changed.push(path.display().to_string());
            edits.push((path, formatted));
        }
    }
    if check {
        if !changed.is_empty() {
            bail!("these files need formatting:\n{}", changed.join("\n"));
        }
    } else {
        for (path, source) in edits {
            write_atomic(path, source.as_bytes())?;
        }
    }
    if json {
        println!(
            "{}",
            serde_json::json!({"command":"fmt", "checked":check, "files":files.len(), "changed":changed})
        );
    } else {
        eprintln!(
            "{} {} files",
            if check {
                "Checked formatting of"
            } else {
                "Formatted"
            },
            files.len()
        );
    }
    io::stdout().flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_options_work_before_and_after_the_subcommand() {
        let before =
            Cli::try_parse_from(["wleam", "--project", "/tmp/project", "build", "--lib"]).unwrap();
        let after =
            Cli::try_parse_from(["wleam", "build", "--lib", "--project", "/tmp/project"]).unwrap();
        assert_eq!(before.project, after.project);
        assert!(matches!(after.command, Action::Build { lib: true, .. }));
    }

    #[test]
    fn atomic_write_replaces_an_existing_file_without_leaving_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("module.wasm");
        write_atomic(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
