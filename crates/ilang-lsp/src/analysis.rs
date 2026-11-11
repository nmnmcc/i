//! Checking a package, on a thread of its own: the server makes a [`Job`]
//! with the text of the open documents, and gets back the [`Analysis`].

use crossbeam_channel::Sender;
use ilang_check::Checked;
use ilang_check::query::{self, Query};
use ilang_package::{Diagnostic, LoadOptions, ModuleKind, ModuleSource, PackageGraph, SourceId};
use lsp_server::{Message, Notification, RequestId};
use lsp_types as lsp;
use lsp_types::notification::Notification as _;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A checked package.
pub(crate) struct Analysis {
    pub graph: PackageGraph,
    pub checked: Checked,
    /// The problems found while loading the packages.
    pub load_diags: Vec<Diagnostic>,
    /// The file of each source, where there is one.
    pub paths: HashMap<SourceId, PathBuf>,
    /// The `i.toml` files whose open document differs from the file. They
    /// are loaded from the file, so their problems are not where they were
    /// found in the document, and are not shown until it is saved.
    pub unsaved_manifests: HashSet<SourceId>,
}

impl Analysis {
    pub fn source_of(&self, path: &Path) -> Option<SourceId> {
        self.paths
            .iter()
            .find(|(_, p)| p.as_path() == path)
            .map(|(s, _)| *s)
    }

    pub fn query(&self) -> Option<Query<'_>> {
        let program = self.checked.program.as_ref()?;
        Some(Query::new(
            &self.graph,
            program,
            self.checked.types.as_ref(),
        ))
    }

    pub fn text(&self, source: SourceId) -> &str {
        let text = &self.graph.sources.get(source).text;
        let original_len = self.checked.program.as_ref().and_then(|p| {
            p.modules
                .iter()
                .find(|m| m.source == source)
                .and_then(|m| m.scoped.as_ref().map(|s| s.original_len() as usize))
        });
        original_len.map_or(text.as_str(), |len| &text[..len])
    }
}

/// A package to check.
pub(crate) struct Job {
    /// The number of the job.
    pub id: u64,
    /// The directory of its `i.toml`.
    pub root: PathBuf,
    /// The changes of the package that the job has seen
    /// (`Server::generations`).
    pub generation: u64,
    /// The text of the open documents, by path.
    pub docs: HashMap<PathBuf, String>,
    pub offline: bool,
    /// Where the files of the library package `i` are shown.
    pub library_dir: Option<PathBuf>,
    /// For a completion after `v.`: the text of one file with a name after
    /// the dot. The analysis only answers that request.
    pub patch: Option<Patch>,
}

pub(crate) struct Patch {
    pub path: PathBuf,
    pub text: String,
    /// The offset of the position of the completion.
    pub offset: u32,
    pub request: RequestId,
}

/// Checks a job on a thread of its own, and sends it back with its
/// analysis. A package that cannot be loaded gives no analysis.
pub(crate) fn spawn(job: Job, done: Sender<(Job, Option<Analysis>)>, log: Sender<Message>) {
    let failed = log.clone();
    let run = move || {
        let mut job = job;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| analyze(&job)));
        let analysis = match result {
            Ok((analysis, true)) => Some(analysis),
            Ok((analysis, false)) => {
                internal_error(&log, &job);
                Some(analysis)
            }
            Err(_) => {
                internal_error(&log, &job);
                None
            }
        };
        job.docs = HashMap::new();
        // The server has stopped when this fails.
        let _ = done.send((job, analysis));
    };
    if let Err(e) = std::thread::Builder::new()
        .name("i-lsp-check".to_string())
        .spawn(run)
    {
        let message = format!("internal error: no thread for a check: {e}");
        let _ = failed.send(log_message(message));
    }
}

fn log_message(message: String) -> Message {
    Message::Notification(Notification::new(
        lsp::notification::LogMessage::METHOD.to_string(),
        lsp::LogMessageParams {
            typ: lsp::MessageType::ERROR,
            message,
        },
    ))
}

fn internal_error(log: &Sender<Message>, job: &Job) {
    let message = format!("internal error: checking `{}` failed", job.root.display());
    let _ = log.send(log_message(message));
}

/// Loads and checks the package of a job, with the text of the open
/// documents, and the patch in place of the text of one file. `false`
/// when the check failed: the analysis has only what was loaded.
fn analyze(job: &Job) -> (Analysis, bool) {
    let env = LoadOptions::from_env();
    let options = LoadOptions {
        tests: true,
        offline: env.offline || job.offline,
        write_lock: false,
        ..env
    };
    let (mut graph, load_diags) = ilang_package::load_with(&job.root, &options);
    let mut paths = HashMap::new();
    for (id, p) in graph.packages.iter().enumerate() {
        let dir = match (&p.dir, &job.library_dir) {
            (Some(dir), _) => dir.clone(),
            (None, Some(library)) if id == graph.library => library.clone(),
            (None, _) => continue,
        };
        if let Some(s) = p.manifest_source {
            paths.insert(s, dir.join("i.toml"));
        }
        for m in &p.modules {
            let rest = m.path.split_once('/').map_or(m.path.as_str(), |x| x.1);
            let extension = if graph.sources.get(m.source).display.ends_with(".is") {
                "is"
            } else {
                "i"
            };
            let file = format!("{rest}.{extension}");
            let path = match m.kind {
                ModuleKind::Src => dir.join("src").join(file),
                ModuleKind::Test => dir.join("test").join(file),
                ModuleKind::Prelude => dir.join(file),
            };
            paths.insert(m.source, path);
        }
    }
    add_new_documents(&job.docs, &mut graph, &mut paths);
    let manifests: HashSet<SourceId> = graph
        .packages
        .iter()
        .filter_map(|p| p.manifest_source)
        .collect();
    let mut unsaved_manifests = HashSet::new();
    for (id, path) in &paths {
        let text = match &job.patch {
            Some(p) if p.path == *path => p.text.as_str(),
            _ => match job.docs.get(path) {
                Some(text) => text.as_str(),
                None => continue,
            },
        };
        if graph.sources.get(*id).text == text {
            continue;
        }
        if manifests.contains(id) {
            unsaved_manifests.insert(*id);
        } else {
            graph.sources.set_text(*id, text.to_string());
        }
    }
    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        query::check_for_queries(&graph)
    }));
    let ok = checked.is_ok();
    let checked = checked.unwrap_or_else(|_| Checked {
        program: None,
        types: None,
        diags: Vec::new(),
    });
    let analysis = Analysis {
        graph,
        checked,
        load_diags,
        paths,
        unsaved_manifests,
    };
    (analysis, ok)
}

/// Adds the open documents under `src/` and `test/` of the package that
/// are not files yet as its modules (§2.1), with their text.
fn add_new_documents(
    docs: &HashMap<PathBuf, String>,
    graph: &mut PackageGraph,
    paths: &mut HashMap<SourceId, PathBuf>,
) {
    let Some(root) = graph.root else { return };
    let Some(dir) = graph.packages[root].dir.clone() else {
        return;
    };
    let mut new: Vec<(&PathBuf, &String)> = docs
        .iter()
        .filter(|(p, _)| p.extension().is_some_and(|e| e == "i" || e == "is"))
        .filter(|(p, _)| !paths.values().any(|q| q == *p))
        .collect();
    new.sort_by(|a, b| a.0.cmp(b.0));
    for (path, text) in new {
        for (sub, kind) in [("src", ModuleKind::Src), ("test", ModuleKind::Test)] {
            let Ok(rel) = path.strip_prefix(dir.join(sub)) else {
                continue;
            };
            let parts: Vec<String> = rel
                .with_extension("")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            if !parts.iter().all(|p| ilang_package::is_name(p)) {
                continue;
            }
            let package = &mut graph.packages[root];
            let module = format!("{}/{}", package.name, parts.join("/"));
            if package.modules.iter().any(|m| m.path == module) {
                continue;
            }
            let extension = path
                .extension()
                .expect("filtered source extension")
                .to_string_lossy();
            let display = format!("{sub}/{}.{extension}", parts.join("/"));
            let source = graph.sources.add(display, text.clone());
            graph.packages[root].modules.push(ModuleSource {
                path: module,
                source,
                kind,
            });
            paths.insert(source, path.clone());
        }
    }
}

#[cfg(test)]
mod expansion_source_tests {
    use super::*;
    use ilang_syntax::lisp::{
        Datum, DatumKind,
        scoped::ScopedSource,
        syntax::{Scope, Syntax},
    };
    use std::collections::BTreeSet;

    #[test]
    fn editor_text_excludes_the_expansion_suffix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let source = "main = () { fake() }\n";
        std::fs::write(dir.path().join("src/main.i"), source).unwrap();
        let (mut graph, _) = ilang_package::load(dir.path());
        let (mut parsed, _) = ilang_check::parse(&graph);
        let module = parsed.iter_mut().find(|m| m.path == "app/main").unwrap();
        let id = module.source;
        let ast = &mut module.ast.items[0];
        let ilang_syntax::ast::ItemKind::Binding(binding) = &mut ast.kind else {
            panic!()
        };
        let ilang_syntax::ast::ExprKind::Fn(function) = &mut binding.value.kind else {
            panic!()
        };
        let Some(ilang_syntax::ast::Stmt::Expr(last)) = function.body.stmts.last_mut() else {
            panic!()
        };
        let call = Syntax::from_datum(
            &Datum {
                kind: DatumKind::Name("fake".into()),
                span: last.span,
                docs: vec![],
            },
            &Scope {
                module: id.0,
                marks: BTreeSet::new(),
            },
        );
        let result = Syntax::from_datum(
            &Datum {
                kind: DatumKind::UpName("Nil".into()),
                span: last.span,
                docs: vec![],
            },
            &Scope {
                module: id.0,
                marks: BTreeSet::new(),
            },
        );
        let mut scoped = ScopedSource::new(source.into());
        let expanded = scoped.expression(&result, &call).unwrap();
        scoped.inherit_call_chain(expanded.span, last.span, "fake");
        *last = expanded;
        graph.sources.set_text(id, scoped.text().into());
        module.scoped = Some(scoped);
        let checked = query::check_parsed_for_queries(&graph, &parsed, vec![]);
        let analysis = Analysis {
            graph,
            checked,
            load_diags: vec![],
            paths: HashMap::new(),
            unsaved_manifests: HashSet::new(),
        };
        assert_eq!(analysis.text(id), source);
        assert!(analysis.graph.sources.get(id).text.len() > source.len());
    }
}
