//! `i fmt [--check] [PATH…]`: formats source files with
//! `ilang_syntax::format`.
//!
//! With no paths, every `.i` and `.is` file under the `src/` and `test/` directories of
//! the package is formatted. A path may be a file or a directory, which is
//! searched for `.i` and `.is` files. A file with syntax errors is left as it is, and
//! its errors are printed. With `--check`, no file is written; the files that
//! would change are listed, and the exit code is 1 when there are any.
//!
//! `--to <surface>` converts instead: it writes the file with the other extension
//! and removes the source. Conversion is done in two phases over the whole
//! run: every file is checked and rendered first, and only when all of them pass
//! does the run write anything. A conversion that would lose a comment or change
//! the module is refused, and nothing is written. A new file is created without
//! clobbering an existing one, synced to disk, read back and compared with the
//! rendered bytes, and only then is the source removed.

use ilang_syntax::format::format;
use ilang_syntax::span::{LineIndex, Span};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run(
    project: &Path,
    check: bool,
    to: Option<&str>,
    paths: &[PathBuf],
) -> anyhow::Result<ExitCode> {
    let mut files = Vec::new();
    if paths.is_empty() {
        let roots = [project.join("src"), project.join("test")];
        if !roots.iter().any(|r| r.is_dir()) {
            anyhow::bail!("{} has no `src/` or `test/` directory", project.display());
        }
        for root in roots {
            collect(&root, &mut files)?;
        }
    } else {
        for path in paths {
            if path.is_dir() {
                collect(path, &mut files)?;
            } else if path.is_file() {
                files.push(path.clone());
            } else {
                anyhow::bail!("{} is not a file or a directory", path.display());
            }
        }
    }
    files.sort();
    files.dedup();

    // A conversion run is planned for the whole file set before anything is
    // written. The ordinary formatting run keeps its one-file-at-a-time shape.
    if to.is_some() {
        return convert_run(&files, to.unwrap(), check);
    }

    let mut failed = false;
    for file in &files {
        let source = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", file.display()))?;
        let is_lisp = file.extension().is_some_and(|e| e == "is");
        let result = if is_lisp {
            ilang_syntax::lisp::format(&source)
        } else {
            format(&source)
        };
        match result {
            Ok(out) if out == source => {}
            Ok(out) => {
                if check {
                    println!("{}", file.display());
                    failed = true;
                } else {
                    std::fs::write(file, out)
                        .map_err(|e| anyhow::anyhow!("cannot write {}: {e}", file.display()))?;
                }
            }
            Err(diags) => {
                failed = true;
                report_diags(file, &source, &diags);
            }
        }
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// What a conversion run will do with one file.
struct Plan {
    source: PathBuf,
    destination: PathBuf,
    output: String,
    /// Whether the source is removed once the destination is written. A file
    /// that is already the target surface is formatted in place instead.
    rename_source: bool,
}

fn convert_run(files: &[PathBuf], target: &str, check: bool) -> anyhow::Result<ExitCode> {
    let mut plans: Vec<Plan> = Vec::new();
    let mut failed = false;

    // Phase one: read, check and render every file. Nothing is written here,
    // so a refusal leaves every file exactly as it was.
    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(source) => source,
            Err(e) => {
                eprintln!("{}: error: cannot read: {e}", file.display());
                failed = true;
                continue;
            }
        };
        let is_lisp = file.extension().is_some_and(|e| e == "is");
        let destination = file.with_extension(target);
        if destination == *file {
            // Already the requested surface: this is ordinary formatting.
            let result = if is_lisp {
                ilang_syntax::lisp::format(&source)
            } else {
                format(&source)
            };
            match result {
                Ok(out) if out == source => {}
                Ok(out) => plans.push(Plan {
                    source: file.clone(),
                    destination: destination.clone(),
                    output: out,
                    rename_source: false,
                }),
                Err(diags) => {
                    failed = true;
                    report_diags(file, &source, &diags);
                }
            }
            continue;
        }
        if destination.exists() {
            eprintln!(
                "{}: error: {} already exists; it will not be overwritten",
                file.display(),
                destination.display()
            );
            failed = true;
            continue;
        }
        let conversion = if is_lisp {
            convert_from_lisp(&source)
        } else {
            convert_to_lisp(&source)
        };
        let (out, lost) = match conversion {
            Ok(converted) => converted,
            Err(diags) => {
                failed = true;
                report_diags(file, &source, &diags);
                continue;
            }
        };
        let lines = LineIndex::new(&source);
        if !lost.is_empty() {
            failed = true;
            for span in &lost {
                let (line, col) = lines.line_col(span.start);
                eprintln!(
                    "{}:{line}:{col}: error: converting to `{}` would lose this text: {}",
                    file.display(),
                    target,
                    snippet(&source, *span)
                );
            }
            eprintln!(
                "{}: the source is unchanged; nothing in this run was converted",
                file.display()
            );
            continue;
        }
        // Round trip: go back to the original surface and compare the canonical
        // text with the canonical text of the source. Anything the printer can
        // carry (comments, literal values, attributes, macro spelling) has to
        // survive; otherwise the conversion is refused.
        if let Err((line, col, message)) = round_trip_diff(is_lisp, &source, &out) {
            failed = true;
            eprintln!(
                "{}:{line}:{col}: error: converting to `{}` is not lossless: {message}",
                file.display(),
                target
            );
            eprintln!(
                "{}: the source is unchanged; nothing in this run was converted",
                file.display()
            );
            continue;
        }
        plans.push(Plan {
            source: file.clone(),
            destination,
            output: out,
            rename_source: true,
        });
    }

    if failed {
        // Phase two never starts: no destination was created and no source
        // removed. Say so, and list every file in the batch as not converted.
        eprintln!(
            "no files were converted: the check phase refused the batch and nothing was written"
        );
        eprintln!("files not converted (all left as they were):");
        for file in files {
            eprintln!("  {}", file.display());
        }
        return Ok(ExitCode::FAILURE);
    }

    if check {
        for plan in &plans {
            if plan.rename_source {
                println!(
                    "{} -> {}",
                    plan.source.display(),
                    plan.destination.display()
                );
            } else {
                println!("{}", plan.source.display());
            }
        }
        return Ok(if plans.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }

    // Phase two: write. The source of a converted file stays in place until its
    // destination is durable, complete and read back byte for byte.
    for (index, plan) in plans.iter().enumerate() {
        let step = if plan.rename_source {
            write_conversion(plan)
        } else {
            write_in_place(plan)
        };
        if let Err(e) = step {
            eprintln!("{}: error: {e}", plan.source.display());
            // Say what happened to every file: the ones already written, the
            // one that failed (with the state of its source and destination),
            // and the ones that were never touched.
            report_write_failure(&plans, index);
            let converted = plans[..index]
                .iter()
                .filter(|plan| plan.rename_source)
                .count();
            let formatted = index - converted;
            let summary = if converted > 0 {
                "the files above were already converted"
            } else if formatted > 0 {
                "the files above were already formatted in place"
            } else {
                "no file was converted"
            };
            return Err(anyhow::anyhow!(
                "{}: conversion stopped; {summary}",
                plan.source.display()
            ));
        }
        if plan.rename_source {
            eprintln!(
                "converted {} -> {}",
                plan.source.display(),
                plan.destination.display()
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Create `destination` without clobbering anything, make it durable, read it
/// back, and only then remove `source`.
///
/// The destination path is public: another process may create a file there at
/// any moment. When a step fails after the destination exists, the destination
/// is left exactly as it is (it may be incomplete) and the source is kept; the
/// error says so. Only the temporary file, whose random name this process made
/// with `create_new`, is cleaned up.
fn write_conversion(plan: &Plan) -> anyhow::Result<()> {
    let destination = &plan.destination;
    let dir = parent_dir(destination);
    let mode = source_mode(&plan.source).map_err(|e| {
        anyhow::anyhow!(
            "cannot read the permissions of {}: {e}; the source is unchanged",
            plan.source.display()
        )
    })?;
    let temporary = write_temporary(dir, destination, plan.output.as_bytes(), mode)?;

    // Atomically publish the temporary file at the destination, failing if the
    // destination already exists. `rename` would replace it, so it is not used:
    // on Linux the destination is created with `renameat2(RENAME_NOREPLACE)`;
    // where the kernel or filesystem lacks that, a hard link to the temporary
    // file is the no-clobber primitive. Neither exists on some filesystems, and
    // then the safe answer is to refuse rather than to guess.
    let consumed = match install_no_clobber(&temporary, destination) {
        Ok(consumed) => consumed,
        Err(e) => {
            cleanup_temporary(&temporary);
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                return Err(anyhow::anyhow!(
                    "{} already exists; it will not be overwritten; the source is unchanged",
                    destination.display()
                ));
            }
            return Err(anyhow::anyhow!(
                "cannot create {} safely: {e}; the source is unchanged",
                destination.display()
            ));
        }
    };
    if !consumed {
        cleanup_temporary(&temporary);
    }

    if let Err(e) = sync_dir(dir) {
        return Err(anyhow::anyhow!(
            "cannot sync {}: {e}; {} may be incomplete and was left in place; {} is unchanged",
            dir.display(),
            destination.display(),
            plan.source.display()
        ));
    }
    // Read the destination back and compare it with what was rendered. This
    // proves the bytes that landed are the bytes that were planned; it is not a
    // comparison with the source.
    match std::fs::read(destination) {
        Ok(bytes) if bytes == plan.output.as_bytes() => {}
        Ok(_) => {
            return Err(anyhow::anyhow!(
                "{} does not match the rendered output; it was left in place and \
                 may be incomplete; {} is unchanged",
                destination.display(),
                plan.source.display()
            ));
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "cannot read back {}: {e}; it was left in place and may be incomplete; \
                 {} is unchanged",
                destination.display(),
                plan.source.display()
            ));
        }
    }
    // Persist the directory entry for the destination before the source goes.
    if let Err(e) = sync_dir(dir) {
        return Err(anyhow::anyhow!(
            "cannot sync {}: {e}; {} is complete but was left in place; {} is unchanged",
            dir.display(),
            destination.display(),
            plan.source.display()
        ));
    }
    std::fs::remove_file(&plan.source).map_err(|e| {
        anyhow::anyhow!(
            "the destination {} is complete but the source cannot be removed: {e}; \
             both files exist",
            destination.display()
        )
    })?;
    let source_dir = parent_dir(&plan.source);
    if let Err(e) = sync_dir(source_dir) {
        eprintln!("warning: cannot sync {}: {e}", source_dir.display());
    }
    Ok(())
}

/// Rewrite a file that is already the requested surface. The new bytes go to a
/// temporary file, are read back, and replace the file with one rename, so a
/// failure leaves the original file intact. The file's content differs (the
/// plan skips unchanged files).
fn write_in_place(plan: &Plan) -> anyhow::Result<()> {
    let dir = parent_dir(&plan.source);
    let mode = source_mode(&plan.source).map_err(|e| {
        anyhow::anyhow!(
            "cannot read the permissions of {}: {e}; the file is unchanged",
            plan.source.display()
        )
    })?;
    let temporary = write_temporary(dir, &plan.source, plan.output.as_bytes(), mode)?;
    // Read back the temporary file before it replaces anything.
    match std::fs::read(&temporary) {
        Ok(bytes) if bytes == plan.output.as_bytes() => {}
        Ok(_) => {
            cleanup_temporary(&temporary);
            return Err(anyhow::anyhow!(
                "the temporary file for {} does not match the rendered output; \
                 the file is unchanged",
                plan.source.display()
            ));
        }
        Err(e) => {
            cleanup_temporary(&temporary);
            return Err(anyhow::anyhow!(
                "cannot read back the temporary file for {}: {e}; the file is unchanged",
                plan.source.display()
            ));
        }
    }
    if let Err(e) = std::fs::rename(&temporary, &plan.source) {
        cleanup_temporary(&temporary);
        return Err(anyhow::anyhow!(
            "cannot replace {}: {e}; the file is unchanged",
            plan.source.display()
        ));
    }
    if let Err(e) = sync_dir(dir) {
        eprintln!("warning: cannot sync {}: {e}", dir.display());
    }
    Ok(())
}

/// Write `bytes` to a fresh temporary file beside `destination`, sync it, and
/// return its path. The name is random and created with `create_new`, so the
/// file is ours alone; on failure it is cleaned up.
fn write_temporary(
    dir: &Path,
    destination: &Path,
    bytes: &[u8],
    mode: Option<u32>,
) -> anyhow::Result<PathBuf> {
    let file_name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_owned());
    for attempt in 0..1024u32 {
        let candidate = dir.join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        // Set the source's mode at creation, so even before writing the file is
        // never more readable than the source. The mode is set again below, in
        // case the process umask narrowed it.
        #[cfg(unix)]
        if let Some(mode) = mode {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(mode);
        }
        let mut file = match options.open(&candidate) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "cannot create a temporary file in {}: {e}",
                    dir.display()
                ));
            }
        };
        if let Err(e) = file.write_all(bytes) {
            drop(file);
            cleanup_temporary(&candidate);
            return Err(anyhow::anyhow!("cannot write {}: {e}", candidate.display()));
        }
        if let Err(e) = file.sync_all() {
            drop(file);
            cleanup_temporary(&candidate);
            return Err(anyhow::anyhow!("cannot sync {}: {e}", candidate.display()));
        }
        drop(file);
        #[cfg(unix)]
        if let Some(mode) = mode {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) =
                std::fs::set_permissions(&candidate, std::fs::Permissions::from_mode(mode))
            {
                cleanup_temporary(&candidate);
                return Err(anyhow::anyhow!(
                    "cannot set the permissions of {}: {e}",
                    candidate.display()
                ));
            }
        }
        // On non-Unix systems the mode is not carried over: there is no portable
        // mode to read, and `set_permissions` only toggles the read-only flag.
        return Ok(candidate);
    }
    Err(anyhow::anyhow!(
        "cannot create a unique temporary file in {}",
        dir.display()
    ))
}

/// Remove a temporary file this process created. Failure is reported but never
/// affects the source or the destination.
fn cleanup_temporary(temporary: &Path) {
    if let Err(e) = std::fs::remove_file(temporary) {
        eprintln!(
            "warning: cannot remove temporary file {}: {e}",
            temporary.display()
        );
    }
}

/// Publish `temporary` at `destination`, failing with `AlreadyExists` instead
/// of replacing anything. Returns whether the temporary path was consumed.
fn install_no_clobber(temporary: &Path, destination: &Path) -> std::io::Result<bool> {
    #[cfg(target_os = "linux")]
    {
        match rename_noreplace(temporary, destination) {
            Ok(()) => return Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Err(e),
            // ENOSYS/EINVAL: this kernel or filesystem has no RENAME_NOREPLACE.
            Err(e) if matches!(e.raw_os_error(), Some(38 | 22 | 95)) => {}
            Err(e) => return Err(e),
        }
    }
    match std::fs::hard_link(temporary, destination) {
        Ok(()) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(e),
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!(
                "this filesystem supports neither `renameat2(RENAME_NOREPLACE)` \
                 nor hard links ({e}); it cannot create the destination without \
                 risking a concurrent file"
            ),
        )),
    }
}

/// `renameat2(RENAME_NOREPLACE)`: an atomic rename that fails with `EEXIST`
/// instead of replacing the destination.
#[cfg(target_os = "linux")]
fn rename_noreplace(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let from = CString::new(from.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let to = CString::new(to.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    // SAFETY: both pointers are valid NUL-terminated strings for the call, and
    // `AT_FDCWD` asks to resolve them relative to the current directory.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// The directory that holds `path`. A bare file name has an empty parent,
/// which is not a usable directory; it means the current directory.
fn parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// The permission bits of `path` on Unix, or `None` elsewhere.
#[cfg(unix)]
fn source_mode(path: &Path) -> std::io::Result<Option<u32>> {
    use std::os::unix::fs::PermissionsExt;
    Ok(Some(std::fs::metadata(path)?.permissions().mode() & 0o7777))
}

#[cfg(not(unix))]
fn source_mode(_path: &Path) -> std::io::Result<Option<u32>> {
    Ok(None)
}

/// Sync a directory entry. Directories cannot be opened as files on every
/// platform; the failure is ignored there because the data file itself was
/// already synced.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Check that the conversion is a fixed point on the target surface: render
/// the source to the target surface, convert that back, render to the target
/// surface again, and require the two renderings to be equal.
///
/// The two surfaces necessarily differ in layout, token spelling and source
/// spans, so those are not compared. What the formatter can carry -- comments,
/// string and literal values, attribute and macro spelling -- does change the
/// rendering, so a conversion that loses any of them is refused. Comments
/// without a place in the new surface are reported and refused separately by
/// the `lost` spans of the conversion, which give file, line and text.
///
/// Known limits: this compares syntax, not evaluation, so a construct that
/// prints the same but means something different to a later stage would not be
/// caught; and it is a check of this printer's round trip, not a proof about
/// ILang semantics.
fn round_trip_diff(is_lisp: bool, _source: &str, rendered: &str) -> Result<(), (u32, u32, String)> {
    // Convert the rendered text back to the source surface and then to the
    // target surface once more; the second rendering must equal the first.
    let again = if is_lisp {
        // Source is Lisp, so `rendered` is normal text. normal -> Lisp -> normal.
        let (lisp, _) = ilang_syntax::syn3::surface::normal_to_lisp(rendered).map_err(|diags| {
            render_error("the converted module cannot be rendered back", &diags)
        })?;
        let (normal, _) = ilang_syntax::syn3::surface::lisp_to_normal(&lisp).map_err(|diags| {
            render_error("the converted module cannot be rendered back", &diags)
        })?;
        format(&normal).map_err(|diags| render_error("the round trip cannot be printed", &diags))?
    } else {
        // Source is normal, so `rendered` is Lisp text. Lisp -> normal -> Lisp.
        let (normal, _) =
            ilang_syntax::syn3::surface::lisp_to_normal(rendered).map_err(|diags| {
                render_error("the converted module cannot be rendered back", &diags)
            })?;
        let (lisp, _) = ilang_syntax::syn3::surface::normal_to_lisp(&normal).map_err(|diags| {
            render_error("the converted module cannot be rendered back", &diags)
        })?;
        ilang_syntax::lisp::format(&lisp)
            .map_err(|diags| render_error("the round trip cannot be printed", &diags))?
    };
    if again == rendered {
        return Ok(());
    }
    let a: Vec<&str> = rendered.lines().collect();
    let b: Vec<&str> = again.lines().collect();
    for (index, (left, right)) in a.iter().zip(b.iter()).enumerate() {
        if left != right {
            return Err((
                (index + 1) as u32,
                1,
                format!(
                    "line {} becomes {right:?} after a round trip, not {left:?}",
                    index + 1
                ),
            ));
        }
    }
    Err((
        (a.len().min(b.len()) + 1) as u32,
        1,
        format!("a round trip has {} lines, not {}", b.len(), a.len()),
    ))
}

fn render_error(what: &str, diags: &[ilang_syntax::Diagnostic]) -> (u32, u32, String) {
    (1, 1, format!("{what}: {}", diag_message(diags)))
}

fn diag_message(diags: &[ilang_syntax::Diagnostic]) -> String {
    diags
        .first()
        .map(|d| d.message.clone())
        .unwrap_or_else(|| "unknown error".to_owned())
}

/// Report the three groups of a run that failed while writing: what was
/// written, what failed and the state of both of its files, and what was not
/// processed.
fn report_write_failure(plans: &[Plan], failed: usize) {
    let converted: Vec<&Plan> = plans[..failed]
        .iter()
        .filter(|plan| plan.rename_source)
        .collect();
    let formatted: Vec<&Plan> = plans[..failed]
        .iter()
        .filter(|plan| !plan.rename_source)
        .collect();
    if converted.is_empty() && formatted.is_empty() {
        eprintln!("already written in this run: none");
    } else {
        eprintln!("already converted in this run (destination written, source removed):");
        if converted.is_empty() {
            eprintln!("  none");
        }
        for plan in &converted {
            eprintln!(
                "  {} -> {}",
                plan.source.display(),
                plan.destination.display()
            );
        }
        if !formatted.is_empty() {
            eprintln!("already formatted in place:");
            for plan in &formatted {
                eprintln!("  {}", plan.source.display());
            }
        }
    }
    let plan = &plans[failed];
    eprintln!("failed:");
    eprintln!(
        "  {} -> {}: source {}, destination {}",
        plan.source.display(),
        plan.destination.display(),
        if plan.source.exists() {
            "is unchanged"
        } else {
            "was removed"
        },
        if plan.destination.exists() {
            "exists and may be incomplete"
        } else {
            "was not written"
        }
    );
    eprintln!("not processed (left as they were):");
    if failed + 1 >= plans.len() {
        eprintln!("  none");
    }
    for plan in &plans[failed + 1..] {
        eprintln!("  {}", plan.source.display());
    }
    let old_extension = plans
        .first()
        .and_then(|plan| plan.source.extension())
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_else(|| "i".to_owned());
    if !converted.is_empty() {
        eprintln!(
            "recover each converted file by converting it back (`i fmt --to {old_extension} DESTINATION`) \
             or by restoring the source from version control"
        );
    }
}

fn report_diags(file: &Path, source: &str, diags: &[ilang_syntax::Diagnostic]) {
    let lines = LineIndex::new(source);
    for d in diags {
        let (line, col) = lines.line_col(d.span.start);
        eprintln!(
            "{}:{line}:{col}: error[{}]: {}",
            file.display(),
            d.code.as_str(),
            d.message
        );
    }
}

/// The source text a span covers, quoted for a one-line message.
fn snippet(source: &str, span: Span) -> String {
    let start = (span.start as usize).min(source.len());
    let end = (span.end as usize).min(source.len());
    let text = source.get(start..end).unwrap_or("").trim();
    let first = text.lines().next().unwrap_or("");
    let mut quoted = format!("{first:?}");
    if first.len() < text.len() {
        quoted.push_str(" (and more)");
    }
    quoted
}

/// The `.i` and `.is` files under `dir`, if it is there.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "i" || e == "is") {
            out.push(path);
        }
    }
    Ok(())
}

/// The other surface of a file, read and written as the tree of syntax
/// three. The errors of the lowering are reported as `i check` reports them.
fn convert_to_lisp(
    source: &str,
) -> Result<(String, Vec<ilang_syntax::span::Span>), Vec<ilang_syntax::Diagnostic>> {
    let (_, errors) = ilang_syntax::parse_module(source);
    if !errors.is_empty() {
        return Err(errors);
    }
    let (out, lost) = ilang_syntax::syn3::surface::normal_to_lisp(source)?;
    Ok((ilang_syntax::lisp::format(&out)?, lost))
}

fn convert_from_lisp(
    source: &str,
) -> Result<(String, Vec<ilang_syntax::span::Span>), Vec<ilang_syntax::Diagnostic>> {
    let (_, errors) = ilang_syntax::lisp::parse_source(source);
    if !errors.is_empty() {
        return Err(errors);
    }
    let (out, lost) = ilang_syntax::syn3::surface::lisp_to_normal(source)?;
    Ok((format(&out)?, lost))
}
