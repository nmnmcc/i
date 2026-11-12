//! Syntax three, stage 4 (docs/design/syntax-3.md §7.2 row 4, §7.3): labels
//! in today's syntax. Resolver-aware: each package is checked with
//! `ilang_check::syn3_facts` on, and the written places it records are
//! rewritten by span, as small textual edits:
//!
//! - a call of a function that is not a constructor with arguments by label
//!   becomes a call by position, in parameter order; when that order would
//!   change the order in which arguments that are not direct values are
//!   worked out (spec §6.2, §6.10), the arguments written first are bound to
//!   fresh `_syn3_N` locals in a group block;
//! - a constructor call gives every labelled field by label;
//! - a part of a constructor pattern matched by label writes its label
//!   (`name = name`, no punning);
//! - labels of the parameters of function types are removed.
//!
//! Usage: `migrate-syntax3-labels [--check] [--lib] PROJECT...`. A project is
//! a directory with an `i.toml`, or a single `.i` file that is checked as
//! `src/main.i` of a package `spec` (as the spec suite does). Only files of
//! the project's own package are edited; with `--lib`, the modules of the
//! library `i` (`lib/`) too, read from disk. `--check` edits nothing and
//! exits 1 when an edit is due. Running it again changes nothing. A file
//! that contains `syn3-labels: keep` is not edited.

use ilang_check::syn3_facts::{self, ArgFact, Fact};
use ilang_syntax::span::Span;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// A file with this text is left as it is (a case that tests the error).
const KEEP: &str = "syn3-labels: keep";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Edit {
    start: usize,
    end: usize,
    text: String,
}

/// Edits that are applied together.
struct Group {
    file: PathBuf,
    edits: Vec<Edit>,
    /// A call or pattern edit, which must land before function type labels
    /// are removed.
    call: bool,
}

fn main() {
    let mut check = false;
    let mut lib = false;
    let mut projects = Vec::new();
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--check" => check = true,
            "--lib" => lib = true,
            _ => projects.push(PathBuf::from(a)),
        }
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let repo = repo.canonicalize().expect("repository root");
    syn3_facts::enable();
    let mut due = 0;
    for project in &projects {
        due += migrate(&repo, project, check, lib);
    }
    if check && due > 0 {
        eprintln!("{due} edits are due");
        std::process::exit(1);
    }
}

/// Runs rounds until no edit is due; gives the number of edits due (with
/// `check`, of the first round).
fn migrate(repo: &Path, project: &Path, check: bool, lib: bool) -> usize {
    let mut total = 0;
    for round in 0..12 {
        let groups = round_groups(repo, project, lib);
        if groups.is_empty() {
            return total;
        }
        let n: usize = groups.iter().map(|g| g.edits.len()).sum();
        if check {
            for g in &groups {
                for e in &g.edits {
                    println!("{}:{}: {:?}", g.file.display(), e.start, e.text);
                }
            }
            return n;
        }
        total += n;
        apply(groups);
        if round == 11 {
            eprintln!("{}: edits still due after 12 rounds", project.display());
        }
    }
    total
}

fn round_groups(repo: &Path, project: &Path, lib: bool) -> Vec<Group> {
    let single = project.is_file() && project.extension().is_some_and(|e| e == "i");
    if single && std::fs::read_to_string(project).is_ok_and(|t| t.contains(KEEP)) {
        return Vec::new();
    }
    let temp;
    let (dir, single_file) = if single {
        temp = tempfile::tempdir().expect("temporary directory");
        let text = std::fs::read_to_string(project).expect("read case");
        let manifest = if text.contains("std/test") {
            format!(
                "name = \"spec\"\n[test]\ncollection = \"std/test.tests\"\n[dependencies]\nstd = {{ path = {:?} }}\n",
                repo.join("std").to_str().unwrap()
            )
        } else {
            "name = \"spec\"\n".to_string()
        };
        std::fs::write(temp.path().join("i.toml"), manifest).unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/main.i"), text).unwrap();
        (temp.path().to_path_buf(), Some(project.to_path_buf()))
    } else {
        (project.to_path_buf(), None)
    };
    let (mut graph, _) = ilang_cli::load(&dir, true, true);
    let Some(root) = graph.root else {
        eprintln!("{}: no package", project.display());
        return Vec::new();
    };
    // The library is compiled into the tool: read it from disk.
    let mut paths: HashMap<u32, PathBuf> = HashMap::new();
    for (id, package) in graph.packages.clone().iter().enumerate() {
        for m in &package.modules {
            let shown = graph.sources.get(m.source).display.clone();
            let path = if let Some(file) = shown.strip_prefix("<i>/") {
                if !lib {
                    continue;
                }
                let a = repo.join("lib/i").join(file);
                let path = if a.exists() {
                    a
                } else {
                    repo.join("lib").join(file)
                };
                let text = std::fs::read_to_string(&path).expect("library module");
                graph.sources.set_text(m.source, text);
                path
            } else if id == root && !graph.sources.get(m.source).text.contains(KEEP) {
                match &single_file {
                    Some(f) => f.clone(),
                    None => {
                        let p = Path::new(&shown);
                        let p = if p.is_absolute() {
                            p.to_path_buf()
                        } else {
                            dir.join(p)
                        };
                        if !p.exists() {
                            eprintln!("{}: cannot find {shown}", project.display());
                            continue;
                        }
                        p
                    }
                }
            } else {
                continue;
            };
            // `.is` modules are in the Lisp form: not rewritten here.
            if path.extension().is_some_and(|e| e == "i") {
                paths.insert(m.source.0, path);
            }
        }
    }
    syn3_facts::take();
    let checked = ilang_macro::project::check(&mut graph);
    let facts = syn3_facts::take();
    let Some(program) = &checked.program else {
        eprintln!("{}: not resolved", project.display());
        return Vec::new();
    };
    let mut fresh: HashMap<PathBuf, u32> = HashMap::new();
    let mut groups = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for fact in facts {
        let file = match &fact {
            Fact::Call { file, .. }
            | Fact::PatternPart { file, .. }
            | Fact::FnTypeLabel { file, .. } => file,
        };
        let Some(m) = program
            .modules
            .iter()
            .find(|m| graph.sources.get(m.source).display == file.shown)
        else {
            continue;
        };
        let Some(path) = paths.get(&m.source.0) else {
            continue;
        };
        let text = graph.sources.get(m.source).text.clone();
        if text.len() != file.len as usize {
            continue;
        }
        // A span of expanded code is mapped to the written code it was
        // spliced from, when it was spliced unchanged.
        let map = |s: Span| -> Option<Span> {
            match &m.scoped {
                Some(scoped) if scoped.is_generated(s) => {
                    let origin = scoped.origin(s)?;
                    let scope = origin.syntax.scope();
                    let o = origin.syntax.span();
                    (scope.module == m.source.0
                        && scope.marks.is_empty()
                        && o.end as usize <= text.len())
                    .then_some(o)
                }
                _ => Some(s),
            }
        };
        let group = match fact {
            Fact::Call {
                span,
                callee,
                callee_direct,
                ctor,
                first,
                args,
                params,
                to,
                ..
            } => {
                let generated = m.scoped.as_ref().is_some_and(|sc| sc.is_generated(span));
                let mapped = (|| {
                    if generated {
                        // Read the arguments again from the written call.
                        let span = map(span)?;
                        let callee = map(callee)?;
                        let first = match first {
                            Some((s, d)) => Some((map(s)?, d)),
                            None => None,
                        };
                        if first.is_none() && callee.start != span.start {
                            return None;
                        }
                        let found = written_args(&text, callee.end as usize, span.end as usize)?;
                        if found.len() != args.len()
                            || found
                                .iter()
                                .zip(&args)
                                .any(|(f, a)| f.label.is_some() != a.label.is_some())
                        {
                            return None;
                        }
                        let args = found
                            .into_iter()
                            .zip(&args)
                            .map(|(f, a)| ArgFact {
                                direct: a.direct,
                                ..f
                            })
                            .collect();
                        return Some((span, callee, first, args));
                    }
                    let span = map(span)?;
                    let callee = map(callee)?;
                    let first = match first {
                        Some((s, d)) => Some((map(s)?, d)),
                        None => None,
                    };
                    let args = args
                        .iter()
                        .map(|a| {
                            Some(ArgFact {
                                label: match a.label {
                                    Some(l) => Some(map(l)?),
                                    None => None,
                                },
                                value: map(a.value)?,
                                direct: a.direct,
                            })
                        })
                        .collect::<Option<Vec<_>>>()?;
                    Some((span, callee, first, args))
                })();
                let mapped = mapped.filter(|(_, _, _, args)| {
                    args.iter().all(|a| {
                        a.label.is_none_or(|l| {
                            text[l.start as usize..l.end as usize]
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
                                && text[l.end as usize..a.value.start as usize].trim() == ":"
                        })
                    })
                });
                let Some((span, callee, first, args)) = mapped else {
                    eprintln!(
                        "{}: a call in expanded code is not migrated",
                        path.display()
                    );
                    continue;
                };
                let counter = fresh.entry(path.clone()).or_insert(0);
                let edits = if ctor {
                    ctor_edits(&text, &args, &params, &to, first.is_some())
                } else {
                    call_edits(
                        &text,
                        span,
                        callee,
                        callee_direct,
                        first,
                        &args,
                        &to,
                        counter,
                    )
                };
                match edits {
                    Ok(edits) => Some(Group {
                        file: path.clone(),
                        edits,
                        call: true,
                    }),
                    Err(why) => {
                        eprintln!(
                            "{}:{}: {why}",
                            path.display(),
                            line(&text, span.start as usize)
                        );
                        None
                    }
                }
            }
            Fact::PatternPart { span, label, .. } => {
                let Some(span) = map(span) else { continue };
                let part = &text[span.start as usize..span.end as usize];
                if !part.starts_with(label.as_str())
                    || part[label.len()..]
                        .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
                {
                    eprintln!(
                        "{}: a pattern in expanded code is not migrated",
                        path.display()
                    );
                    continue;
                }
                let rest = part.strip_prefix(label.as_str()).map(str::trim_start);
                let written = rest.is_some_and(|r| r.starts_with('=') && !r.starts_with("=="));
                (!written).then(|| Group {
                    file: path.clone(),
                    edits: vec![Edit {
                        start: span.start as usize,
                        end: span.start as usize,
                        text: format!("{label} = "),
                    }],
                    call: true,
                })
            }
            Fact::FnTypeLabel { label, ty, .. } => {
                let (Some(label), Some(ty)) = (map(label), map(ty)) else {
                    continue;
                };
                let between = &text[label.end as usize..ty.start as usize];
                let name = &text[label.start as usize..label.end as usize];
                (between.trim() == ":"
                    && !name.is_empty()
                    && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'))
                .then(|| Group {
                    file: path.clone(),
                    edits: vec![Edit {
                        start: label.start as usize,
                        end: ty.start as usize,
                        text: String::new(),
                    }],
                    call: false,
                })
            }
        };
        if let Some(g) = group
            && seen.insert((g.file.clone(), g.edits.clone()))
        {
            groups.push(g);
        }
    }
    select(groups)
}

/// The arguments of the call whose callee ends at `from` and which ends at
/// `end`, read from the text: commas and breaks separate them (spec §6.1).
fn written_args(text: &str, from: usize, end: usize) -> Option<Vec<ArgFact>> {
    let b = text.as_bytes();
    let mut i = from;
    while i < end && b[i].is_ascii_whitespace() {
        i += 1;
    }
    if b.get(i) != Some(&b'(') || b.get(end - 1) != Some(&b')') {
        return None;
    }
    let (open, close) = (i + 1, end - 1);
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = open;
    let mut j = open;
    while j < close {
        match b[j] {
            b'"' => {
                j += 1;
                while j < close && b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
            }
            b'/' if b.get(j + 1) == Some(&b'/') => {
                while j < close && b[j] != b'\n' {
                    j += 1;
                }
                continue;
            }
            b'<' if b.get(j + 1) == Some(&b'<') => {
                depth += 1;
                j += 1;
            }
            b'>' if b.get(j + 1) == Some(&b'>') && depth > 0 => {
                depth -= 1;
                j += 1;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' | b'\n' if depth == 0 => {
                if !text[start..j].trim().is_empty() {
                    parts.push((start, j));
                }
                start = j + 1;
            }
            _ => {}
        }
        j += 1;
    }
    if depth != 0 {
        return None;
    }
    if !text[start..close].trim().is_empty() {
        parts.push((start, close));
    }
    Some(
        parts
            .into_iter()
            .map(|(s, e)| {
                let seg = &text[s..e];
                let s = s + (seg.len() - seg.trim_start().len());
                let e = e - (seg.len() - seg.trim_end().len());
                let seg = &text[s..e];
                let name = seg
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(seg.len());
                let after = seg[name..].trim_start();
                let label = (name > 0
                    && seg.as_bytes()[0].is_ascii_lowercase() | (seg.as_bytes()[0] == b'_')
                    && after.starts_with(':')
                    && !after.starts_with("::")
                    && !after.starts_with(":="))
                .then_some(Span {
                    start: s as u32,
                    end: (s + name) as u32,
                });
                let value_start = match label {
                    Some(_) => {
                        let colon = s + seg.find(':').expect("label colon");
                        colon
                            + 1
                            + (text[colon + 1..e].len() - text[colon + 1..e].trim_start().len())
                    }
                    None => s,
                };
                ArgFact {
                    label,
                    value: Span {
                        start: value_start as u32,
                        end: e as u32,
                    },
                    direct: false,
                }
            })
            .collect(),
    )
}

fn line(text: &str, at: usize) -> usize {
    text[..at.min(text.len())].matches('\n').count() + 1
}

/// Constructor calls: each argument by position to a labelled field gets its
/// label.
fn ctor_edits(
    text: &str,
    args: &[ArgFact],
    params: &[Option<String>],
    to: &[usize],
    piped: bool,
) -> Result<Vec<Edit>, String> {
    let skip = usize::from(piped);
    let mut edits = Vec::new();
    let mut labelled = false;
    for (i, a) in args.iter().enumerate() {
        match (&a.label, &params[to[skip + i]]) {
            (Some(_), _) => labelled = true,
            (None, Some(l)) => {
                labelled = true;
                let at = a.value.start as usize;
                edits.push(Edit {
                    start: at,
                    end: at,
                    text: format!("{l}: "),
                });
            }
            (None, None) if labelled => {
                return Err(format!(
                    "a constructor argument by position after a labelled one: `{}`",
                    &text[a.value.start as usize..a.value.end as usize]
                ));
            }
            (None, None) => {}
        }
    }
    Ok(edits)
}

/// A call of a function by label becomes a call by position.
#[allow(clippy::too_many_arguments)]
fn call_edits(
    text: &str,
    span: Span,
    callee: Span,
    callee_direct: bool,
    first: Option<(Span, bool)>,
    args: &[ArgFact],
    to: &[usize],
    counter: &mut u32,
) -> Result<Vec<Edit>, String> {
    let skip = usize::from(first.is_some());
    let src = |s: Span| text[s.start as usize..s.end as usize].to_string();
    let slot = |a: &ArgFact| Span {
        start: a.label.map_or(a.value.start, |l| l.start),
        end: a.value.end,
    };
    // The written arguments in parameter order.
    let mut sorted: Vec<usize> = (0..args.len()).collect();
    sorted.sort_by_key(|i| to[skip + i]);
    if sorted.iter().enumerate().all(|(k, i)| k == *i) {
        return Ok(args
            .iter()
            .filter_map(|a| {
                a.label.map(|l| Edit {
                    start: l.start as usize,
                    end: a.value.start as usize,
                    text: String::new(),
                })
            })
            .collect());
    }
    // The arguments that run code, in written order; the longest end of
    // them that is already in parameter order stays in place, and the
    // others are bound first.
    let running: Vec<usize> = (0..args.len()).filter(|i| !args[*i].direct).collect();
    let mut keep = running.len();
    while keep > 0
        && (keep == running.len() || to[skip + running[keep - 1]] < to[skip + running[keep]])
    {
        keep -= 1;
    }
    let bound: Vec<usize> = running[..keep].to_vec();
    if bound.is_empty() {
        // Only direct values move: put each value in its new place.
        return Ok(sorted
            .iter()
            .enumerate()
            .filter(|(k, i)| args[*k].label.is_some() || *k != **i)
            .map(|(k, i)| {
                let s = slot(&args[k]);
                Edit {
                    start: s.start as usize,
                    end: s.end as usize,
                    text: src(args[*i].value),
                }
            })
            .collect());
    }
    let mut name = || loop {
        let n = format!("_syn3_{counter}");
        *counter += 1;
        let used = text.match_indices(&n).any(|(at, _)| {
            !text[at + n.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
        });
        if !used {
            return n;
        }
    };
    let mut lets = Vec::new();
    // Inside the call: (span, replacement), not overlapping.
    let mut inner: Vec<(Span, String)> = Vec::new();
    if let Some((f, direct)) = first
        && !direct
    {
        let n = name();
        lets.push(format!("{n} = {}", src(f)));
        inner.push((f, n));
    }
    if !callee_direct {
        let n = name();
        lets.push(format!("{n} = {}", src(callee)));
        inner.push((callee, n));
    }
    let mut names: BTreeMap<usize, String> = BTreeMap::new();
    for i in &bound {
        let n = name();
        lets.push(format!("{n} = {}", src(args[*i].value)));
        names.insert(*i, n);
    }
    for (k, i) in sorted.iter().enumerate() {
        let value = names.get(i).cloned().unwrap_or_else(|| src(args[*i].value));
        inner.push((slot(&args[k]), value));
    }
    inner.sort_by_key(|(s, _)| s.start);
    let mut call = String::new();
    let mut at = span.start;
    for (s, r) in &inner {
        call.push_str(&text[at as usize..s.start as usize]);
        call.push_str(r);
        at = s.end;
    }
    call.push_str(&text[at as usize..span.end as usize]);
    Ok(vec![Edit {
        start: span.start as usize,
        end: span.end as usize,
        text: format!("{{ {}  {call} }}", lets.join("  ")),
    }])
}

fn overlaps(a: &Edit, b: &Edit) -> bool {
    if a.start == a.end && b.start == b.end {
        return a.start == b.start;
    }
    if a.start == a.end {
        return b.start < a.start && a.start < b.end;
    }
    if b.start == b.end {
        return a.start < b.start && b.start < a.end;
    }
    a.start < b.end && b.start < a.end
}

/// Inner groups first; a group that overlaps one taken waits for the next
/// round. Function type labels wait while a call edit waits.
fn select(mut groups: Vec<Group>) -> Vec<Group> {
    let hull = |g: &Group| {
        let s = g.edits.iter().map(|e| e.start).min().unwrap_or(0);
        let e = g.edits.iter().map(|e| e.end).max().unwrap_or(0);
        e - s
    };
    groups.retain(|g| !g.edits.is_empty());
    groups.sort_by_key(hull);
    let mut taken: Vec<Group> = Vec::new();
    let mut waiting = false;
    for g in groups {
        let clash = taken.iter().any(|t| {
            t.file == g.file
                && t.edits
                    .iter()
                    .any(|a| g.edits.iter().any(|b| a != b && overlaps(a, b)))
        });
        if clash {
            waiting |= g.call;
        } else {
            taken.push(g);
        }
    }
    if waiting {
        taken.retain(|g| g.call);
    }
    taken
}

fn apply(groups: Vec<Group>) {
    let mut by_file: HashMap<PathBuf, Vec<Edit>> = HashMap::new();
    for g in groups {
        by_file.entry(g.file).or_default().extend(g.edits);
    }
    for (file, mut edits) in by_file {
        edits.sort_by(|a, b| b.start.cmp(&a.start).then(b.end.cmp(&a.end)));
        edits.dedup();
        let mut text = std::fs::read_to_string(&file).expect("read");
        for e in edits {
            text.replace_range(e.start..e.end, &e.text);
        }
        std::fs::write(&file, text).expect("write");
    }
}
