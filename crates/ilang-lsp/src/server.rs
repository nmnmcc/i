//! The server: the documents that are open, the packages they are in, and
//! the answers to requests.
//!
//! A package is checked as a whole (`ilang_check::query::check_for_queries`),
//! with the text of the open documents in place of the files on disk, on a
//! thread of its own ([`analysis::spawn`]), so that messages are read and
//! answered while it works. A change marks the packages that have the file
//! as changed; they are checked again when no change has come for
//! `Options::debounce`, one check of a package at a time, or at once when
//! a request waits for them, also while a check of older documents goes
//! on.
//!
//! A request is answered from the last check of its package. While that
//! check is older than the document, hover and go to definition are still
//! answered from it where the text around the position has not changed
//! since ([`Change`]); other requests, and those at a changed place, wait
//! for the next check. Completion after `v.` is answered from the last
//! check when it knows `v`, else from a check of the text with a name
//! after the dot.

use crate::analysis::{self, Analysis, Job, Patch};
use crate::change::{self, Change};
use crate::position::LineIndex;
use crate::{format, uri};
use crossbeam_channel::{Receiver, Sender};
use ilang_check::hir::ModuleId;
use ilang_check::query::{self, CompletionKind, SymbolKind};
use ilang_package::{Severity, SourceId};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types as lsp;
use lsp_types::notification::Notification as _;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How the server works.
#[derive(Clone, Debug)]
pub struct Options {
    /// How long to wait after a change before checking again.
    pub debounce: Duration,
    /// Never use the network to get packages. Without it, only the first
    /// check of a package may use it.
    pub offline: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            debounce: Duration::from_millis(200),
            offline: false,
        }
    }
}

/// The capabilities of the server.
pub fn capabilities() -> lsp::ServerCapabilities {
    lsp::ServerCapabilities {
        text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
            lsp::TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(lsp::TextDocumentSyncKind::FULL),
                save: Some(lsp::TextDocumentSyncSaveOptions::Supported(true)),
                ..Default::default()
            },
        )),
        hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
        definition_provider: Some(lsp::OneOf::Left(true)),
        document_symbol_provider: Some(lsp::OneOf::Left(true)),
        completion_provider: Some(lsp::CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
        document_formatting_provider: format::formatter().map(|_| lsp::OneOf::Left(true)),
        ..Default::default()
    }
}

/// Runs the server on a connection: the `initialize` handshake, then
/// messages until `shutdown` and `exit`, or until the connection closes.
pub fn run(conn: &Connection, options: Options) -> anyhow::Result<()> {
    let (id, _params) = conn.initialize_start()?;
    let result = lsp::InitializeResult {
        capabilities: capabilities(),
        server_info: Some(lsp::ServerInfo {
            name: "i".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    };
    conn.initialize_finish(id, serde_json::to_value(result)?)?;
    Server::new(conn, options).main_loop()
}

struct Doc {
    text: String,
    /// The URI that the client uses for the document.
    uri: lsp::Uri,
}

/// The most checks that run at once.
const MAX_JOBS: usize = 3;

/// Why a request is not answered yet.
enum Later {
    /// It waits for a check of the package in this directory that has
    /// seen the documents as they are now.
    Wait(PathBuf),
    /// It is answered from a check of the package in this directory, with
    /// this patch.
    Check(PathBuf, Patch),
}

/// A document position in the last check of its package.
struct Place<'a> {
    root: PathBuf,
    a: &'a Analysis,
    source: SourceId,
    module: ModuleId,
    /// From the text of the check to the text of the document now.
    change: Change,
    /// The text of the document now.
    text: &'a str,
    /// The check has seen the documents as they are now.
    fresh: bool,
}

impl Place<'_> {
    /// The answer when the check cannot answer: none if it is up to date,
    /// else after the next one.
    fn later<T>(&self) -> Result<Option<T>, Later> {
        if self.fresh {
            Ok(None)
        } else {
            Err(Later::Wait(self.root.clone()))
        }
    }

    fn old_text(&self) -> &str {
        self.a.text(self.source)
    }
}

struct Server<'c> {
    conn: &'c Connection,
    options: Options,
    /// The open documents, by path.
    docs: HashMap<PathBuf, Doc>,
    /// The last check of each package, by the directory of its `i.toml`.
    analyses: HashMap<PathBuf, Analysis>,
    /// The number of changes of each package so far. A package is checked
    /// again until a check has seen them all.
    generations: HashMap<PathBuf, u64>,
    /// The generation of each package that its last check has seen.
    checked: HashMap<PathBuf, u64>,
    /// The packages to check without waiting for `Options::debounce`.
    urgent: HashSet<PathBuf>,
    /// When the last change came.
    last_change: Instant,
    /// The requests that wait for a check of their package.
    waiting: Vec<(Request, PathBuf)>,
    /// The checks for completions that are not started yet.
    patches: VecDeque<Job>,
    /// The jobs being checked: the number of each, and for a check of a
    /// package, its directory and the generation it has seen.
    running: Vec<(u64, Option<(PathBuf, u64)>)>,
    next_job: u64,
    done_tx: Sender<(Job, Option<Analysis>)>,
    done: Receiver<(Job, Option<Analysis>)>,
    /// The URIs with diagnostics, by package.
    published: HashMap<PathBuf, Vec<lsp::Uri>>,
    /// The packages that have been checked once.
    loaded: HashSet<PathBuf>,
    /// A new private directory, removed when the server stops, where the
    /// files of the library package `i` are written when a definition is
    /// there. Never a path that another user could know beforehand: they
    /// could put a link there to a file of ours for us to write over.
    library: Option<tempfile::TempDir>,
}

/// The canonical path of a file, also of one that is not on the disk (a
/// new document), from the canonical path of its directory.
fn normalize(path: &Path) -> PathBuf {
    if let Ok(p) = path.canonicalize() {
        return p;
    }
    match (path.parent(), path.file_name()) {
        (Some(dir), Some(name)) => normalize(dir).join(name),
        _ => path.to_path_buf(),
    }
}

/// A new directory that only this user may use.
fn private_dir() -> Option<tempfile::TempDir> {
    let dir = tempfile::Builder::new()
        .prefix("ilang-lsp-library-")
        .tempdir()
        .ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).ok()?;
    }
    Some(dir)
}

/// The directory of the nearest `i.toml` above a file.
fn package_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .skip(1)
        .find(|d| d.join("i.toml").is_file())
        .map(normalize)
}

impl<'c> Server<'c> {
    fn new(conn: &'c Connection, options: Options) -> Server<'c> {
        let (done_tx, done) = crossbeam_channel::unbounded();
        Server {
            conn,
            options,
            docs: HashMap::new(),
            analyses: HashMap::new(),
            generations: HashMap::new(),
            checked: HashMap::new(),
            urgent: HashSet::new(),
            last_change: Instant::now(),
            waiting: Vec::new(),
            patches: VecDeque::new(),
            running: Vec::new(),
            next_job: 0,
            done_tx,
            done,
            published: HashMap::new(),
            loaded: HashSet::new(),
            library: private_dir(),
        }
    }

    fn library_dir(&self) -> Option<&Path> {
        self.library.as_ref().map(tempfile::TempDir::path)
    }

    fn main_loop(&mut self) -> anyhow::Result<()> {
        loop {
            self.start_jobs();
            let timeout = self.next_check().unwrap_or(Duration::from_secs(3600));
            crossbeam_channel::select! {
                recv(self.conn.receiver) -> msg => {
                    let Ok(msg) = msg else { return Ok(()) };
                    match msg {
                        Message::Request(req) => {
                            if self.conn.handle_shutdown(&req)? {
                                return Ok(());
                            }
                            self.request(req);
                        }
                        Message::Notification(n) => {
                            if n.method == lsp::notification::Exit::METHOD {
                                return Ok(());
                            }
                            self.notification(n);
                        }
                        Message::Response(_) => {}
                    }
                }
                recv(self.done) -> done => {
                    if let Ok((job, analysis)) = done {
                        self.finished(job, analysis);
                    }
                }
                default(timeout) => {}
            }
        }
    }

    fn send(&self, msg: Message) {
        // The client has gone when this fails; the loop ends at the next
        // receive.
        let _ = self.conn.sender.send(msg);
    }

    fn notify<N: lsp::notification::Notification>(&self, params: N::Params) {
        self.send(Message::Notification(Notification::new(
            N::METHOD.to_string(),
            params,
        )));
    }

    // ---- Notifications ----

    fn notification(&mut self, n: Notification) {
        use lsp::notification::*;
        match n.method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(p) =
                    n.extract::<lsp::DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
                {
                    let doc = p.text_document;
                    if let Some(path) = uri::to_path(&doc.uri) {
                        let path = normalize(&path);
                        self.docs.insert(
                            path.clone(),
                            Doc {
                                text: doc.text,
                                uri: doc.uri,
                            },
                        );
                        self.changed(&path);
                        // Show the problems of a new document at once.
                        if let Some(root) = package_root(&path)
                            && !self.analyses.contains_key(&root)
                        {
                            self.urgent.insert(root);
                        }
                    }
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(p) =
                    n.extract::<lsp::DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD)
                    && let Some(path) = uri::to_path(&p.text_document.uri)
                {
                    let path = normalize(&path);
                    if let Some(doc) = self.docs.get_mut(&path) {
                        // Full sync: the last change is the whole text.
                        if let Some(c) = p.content_changes.into_iter().last() {
                            doc.text = c.text;
                        }
                    }
                    self.changed(&path);
                }
            }
            DidSaveTextDocument::METHOD => {
                if let Ok(p) =
                    n.extract::<lsp::DidSaveTextDocumentParams>(DidSaveTextDocument::METHOD)
                    && let Some(path) = uri::to_path(&p.text_document.uri)
                {
                    let path = normalize(&path);
                    self.changed(&path);
                    // Check every changed package at once.
                    let stale: Vec<PathBuf> = self
                        .generations
                        .keys()
                        .filter(|r| !self.fresh(r))
                        .cloned()
                        .collect();
                    self.urgent.extend(stale);
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(p) =
                    n.extract::<lsp::DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)
                    && let Some(path) = uri::to_path(&p.text_document.uri)
                {
                    let path = normalize(&path);
                    // The file on disk counts again.
                    self.docs.remove(&path);
                    self.changed(&path);
                }
            }
            DidChangeWatchedFiles::METHOD => {
                if let Ok(p) =
                    n.extract::<lsp::DidChangeWatchedFilesParams>(DidChangeWatchedFiles::METHOD)
                {
                    for e in p.changes {
                        if let Some(path) = uri::to_path(&e.uri) {
                            self.changed(&normalize(&path));
                        }
                    }
                    // A file may be new: check every package again.
                    let roots: Vec<PathBuf> = self.analyses.keys().cloned().collect();
                    for root in roots {
                        self.bump(root);
                    }
                }
            }
            Cancel::METHOD => {
                if let Ok(p) = n.extract::<lsp::CancelParams>(Cancel::METHOD) {
                    let id: RequestId = match p.id {
                        lsp::NumberOrString::Number(n) => n.into(),
                        lsp::NumberOrString::String(s) => s.into(),
                    };
                    self.cancel(&id);
                }
            }
            _ => {}
        }
    }

    /// Marks the packages with this file as changed: its own package, and
    /// the checked packages that use it.
    fn changed(&mut self, path: &Path) {
        let mut roots: Vec<PathBuf> = package_root(path).into_iter().collect();
        for (root, a) in &self.analyses {
            if a.paths.values().any(|p| p == path) || path.ends_with("i.toml") {
                roots.push(root.clone());
            }
        }
        for root in roots {
            self.bump(root);
        }
    }

    fn bump(&mut self, root: PathBuf) {
        *self.generations.entry(root).or_default() += 1;
        self.last_change = Instant::now();
    }

    fn generation(&self, root: &Path) -> u64 {
        self.generations.get(root).copied().unwrap_or(0)
    }

    /// The last check of the package has seen every change.
    fn fresh(&self, root: &Path) -> bool {
        self.checked.get(root) == Some(&self.generation(root))
    }

    /// A request that has not been answered yet is not answered.
    fn cancel(&mut self, id: &RequestId) {
        let before = self.waiting.len() + self.patches.len();
        self.waiting.retain(|(r, _)| r.id != *id);
        self.patches
            .retain(|j| j.patch.as_ref().is_none_or(|p| p.request != *id));
        if self.waiting.len() + self.patches.len() < before {
            self.send(Message::Response(error(
                id.clone(),
                ErrorCode::RequestCanceled,
                "canceled".to_string(),
            )));
        }
    }

    // ---- Checking ----

    /// A check of the package at its generation now is running.
    fn checking(&self, root: &Path) -> bool {
        let generation = self.generation(root);
        self.running.iter().any(|(_, r)| {
            r.as_ref()
                .is_some_and(|(r, g)| r == root && *g == generation)
        })
    }

    /// The changed packages that are checked when no change has come for
    /// `Options::debounce`: one check of a package at a time.
    fn debounced(&self) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = self
            .generations
            .keys()
            .filter(|r| !self.fresh(r))
            .filter(|r| {
                !self
                    .running
                    .iter()
                    .any(|(_, x)| x.as_ref().is_some_and(|(x, _)| x == *r))
            })
            .cloned()
            .collect();
        roots.sort();
        roots
    }

    /// How long until a changed package is checked, when one is waiting.
    fn next_check(&self) -> Option<Duration> {
        if self.debounced().is_empty() {
            return None;
        }
        Some(
            self.options
                .debounce
                .saturating_sub(self.last_change.elapsed()),
        )
    }

    /// Starts the checks to make now: first of the packages that requests
    /// wait for and of new documents, not behind a check of older
    /// documents, which goes on but is not waited for; then the checks for
    /// completions; then of the changed packages, when no change has come
    /// for a while.
    fn start_jobs(&mut self) {
        let mut needed: Vec<PathBuf> = Vec::new();
        for root in self.waiting.iter().map(|(_, r)| r).chain(&self.urgent) {
            if !self.fresh(root) && !self.checking(root) && !needed.contains(root) {
                needed.push(root.clone());
            }
        }
        for root in needed {
            if self.running.len() >= MAX_JOBS {
                return;
            }
            self.start_check(root);
        }
        while self.running.len() < MAX_JOBS
            && let Some(job) = self.patches.pop_front()
        {
            self.start(job);
        }
        if self.last_change.elapsed() >= self.options.debounce {
            for root in self.debounced() {
                if self.running.len() >= MAX_JOBS {
                    return;
                }
                self.start_check(root);
            }
        }
    }

    fn start_check(&mut self, root: PathBuf) {
        self.urgent.remove(&root);
        let job = self.job(root.clone(), None);
        self.loaded.insert(root);
        self.start(job);
    }

    fn start(&mut self, job: Job) {
        let package = job
            .patch
            .is_none()
            .then(|| (job.root.clone(), job.generation));
        self.running.push((job.id, package));
        analysis::spawn(job, self.done_tx.clone(), self.conn.sender.clone());
    }

    /// A job for the package in `root`, with the documents as they are now.
    fn job(&mut self, root: PathBuf, patch: Option<Patch>) -> Job {
        self.next_job += 1;
        Job {
            id: self.next_job,
            generation: self.generation(&root),
            offline: self.options.offline || self.loaded.contains(&root),
            docs: self
                .docs
                .iter()
                .map(|(p, d)| (p.clone(), d.text.clone()))
                .collect(),
            library_dir: self.library_dir().map(Path::to_path_buf),
            root,
            patch,
        }
    }

    /// A job is done: keep the check of a package and publish its
    /// problems, or answer the completion that it was for.
    fn finished(&mut self, job: Job, analysis: Option<Analysis>) {
        self.running.retain(|(id, _)| *id != job.id);
        if let Some(patch) = job.patch {
            let items = analysis.and_then(|b| {
                let q = b.query()?;
                let module = q.module_of_source(b.source_of(&patch.path)?)?;
                Some(q.completions_at(module, patch.offset))
            });
            self.send(Message::Response(ok(
                patch.request,
                items.map(completion_response),
            )));
            return;
        }
        let root = job.root;
        // A check that ends after a check of newer documents is dropped.
        if self.checked.get(&root).is_some_and(|g| *g > job.generation) {
            return;
        }
        self.checked.insert(root.clone(), job.generation);
        match analysis {
            Some(a) => {
                // A check of documents that have changed since is not
                // published: the next one will be.
                if job.generation == self.generation(&root) {
                    self.publish(&root, &a);
                }
                self.analyses.insert(root, a);
            }
            None => {
                self.analyses.remove(&root);
            }
        }
        // The requests that wait for the package.
        for (req, _) in std::mem::take(&mut self.waiting) {
            self.answer(req);
        }
    }

    fn uri_of(&self, path: &Path) -> Option<lsp::Uri> {
        match self.docs.get(path) {
            Some(d) => Some(d.uri.clone()),
            None => uri::from_path(path),
        }
    }

    fn publish(&mut self, root: &Path, a: &Analysis) {
        let mut by_file: HashMap<PathBuf, Vec<lsp::Diagnostic>> = HashMap::new();
        let library = a.graph.library;
        let library_sources: HashSet<SourceId> = a.graph.packages[library]
            .modules
            .iter()
            .map(|m| m.source)
            .collect();
        let mut seen = HashSet::new();
        for d in a.load_diags.iter().chain(&a.checked.diags) {
            if d.source
                .is_some_and(|s| library_sources.contains(&s) || a.unsaved_manifests.contains(&s))
            {
                continue;
            }
            let (path, range) = match d.source.and_then(|s| a.paths.get(&s).map(|p| (s, p))) {
                Some((s, p)) => (p.clone(), LineIndex::new(a.text(s)).range(d.span)),
                None => (root.join("i.toml"), lsp::Range::default()),
            };
            if !seen.insert((path.clone(), d.span, d.code, d.message.clone())) {
                continue;
            }
            by_file.entry(path).or_default().push(lsp::Diagnostic {
                range,
                severity: Some(match d.severity {
                    Severity::Error => lsp::DiagnosticSeverity::ERROR,
                    Severity::Warning => lsp::DiagnosticSeverity::WARNING,
                }),
                code: Some(lsp::NumberOrString::String(d.code.to_string())),
                source: Some("i".to_string()),
                message: d.message.clone(),
                related_information: (!d.related.is_empty()).then(|| {
                    d.related
                        .iter()
                        .filter_map(|note| {
                            let path = a.paths.get(&note.source)?;
                            let uri = self.uri_of(path)?;
                            Some(lsp::DiagnosticRelatedInformation {
                                location: lsp::Location {
                                    uri,
                                    range: LineIndex::new(a.text(note.source)).range(note.span),
                                },
                                message: note.message.clone(),
                            })
                        })
                        .collect()
                }),
                ..Default::default()
            });
        }
        let mut now = Vec::new();
        for (path, mut diags) in by_file {
            let Some(uri) = self.uri_of(&path) else {
                continue;
            };
            diags.sort_by_key(|d| (d.range.start.line, d.range.start.character));
            now.push(uri.clone());
            self.notify::<lsp::notification::PublishDiagnostics>(lsp::PublishDiagnosticsParams {
                uri,
                diagnostics: diags,
                version: None,
            });
        }
        // Clear the files that had problems and have none now, and the open
        // documents of the package.
        let mut clear: Vec<lsp::Uri> = self
            .published
            .get(root)
            .map(|s| s.iter().filter(|u| !now.contains(u)).cloned().collect())
            .unwrap_or_default();
        for (path, doc) in &self.docs {
            if a.paths.values().any(|p| p == path)
                && !now.contains(&doc.uri)
                && !clear.contains(&doc.uri)
            {
                clear.push(doc.uri.clone());
            }
        }
        for uri in clear {
            self.notify::<lsp::notification::PublishDiagnostics>(lsp::PublishDiagnosticsParams {
                uri,
                diagnostics: Vec::new(),
                version: None,
            });
        }
        self.published.insert(root.to_path_buf(), now);
    }

    // ---- Requests ----

    fn request(&mut self, req: Request) {
        use lsp::request::{Formatting, Request as _};
        if req.method == Formatting::METHOD {
            let id = req.id.clone();
            let response = match req.extract::<lsp::DocumentFormattingParams>(Formatting::METHOD) {
                Ok((id, p)) => self.formatting(id, p),
                Err(e) => error(id, ErrorCode::InvalidParams, e.to_string()),
            };
            return self.send(Message::Response(response));
        }
        self.answer(req);
    }

    /// Answers a request now, or puts it aside until it can be.
    fn answer(&mut self, req: Request) {
        use lsp::request::*;
        let id = req.id.clone();
        let result = match req.method.as_str() {
            HoverRequest::METHOD => params(&req, |p: lsp::HoverParams| {
                self.hover(p.text_document_position_params)
            }),
            GotoDefinition::METHOD => params(&req, |p: lsp::GotoDefinitionParams| {
                self.definition(p.text_document_position_params)
            }),
            DocumentSymbolRequest::METHOD => {
                params(&req, |p: lsp::DocumentSymbolParams| self.symbols(p))
            }
            Completion::METHOD => params(&req, |p: lsp::CompletionParams| {
                self.completion(&id, p.text_document_position)
            }),
            _ => Ok(Err(error(
                id.clone(),
                ErrorCode::MethodNotFound,
                format!("unknown request `{}`", req.method),
            ))),
        };
        let response = match result {
            Ok(Ok(value)) => Response::new_ok(id, value),
            Ok(Err(response)) => response,
            Err(Later::Wait(root)) => {
                self.waiting.push((req, root));
                return;
            }
            Err(Later::Check(root, patch)) => {
                let job = self.job(root, Some(patch));
                self.patches.push_back(job);
                return;
            }
        };
        self.send(Message::Response(response));
    }

    /// The document at a URI, in the last check of its package.
    fn place(&self, uri: &lsp::Uri) -> Result<Option<Place<'_>>, Later> {
        let Some(path) = uri::to_path(uri).map(|p| normalize(&p)) else {
            return Ok(None);
        };
        let Some(root) = package_root(&path) else {
            return Ok(None);
        };
        let fresh = self.fresh(&root);
        let found = self.analyses.get(&root).and_then(|a| {
            let source = a.source_of(&path)?;
            let module = a.query()?.module_of_source(source)?;
            Some((a, source, module))
        });
        let Some((a, source, module)) = found else {
            return if fresh {
                Ok(None)
            } else {
                Err(Later::Wait(root))
            };
        };
        let text = match self.docs.get(&path) {
            Some(d) => d.text.as_str(),
            None => a.text(source),
        };
        Ok(Some(Place {
            change: Change::new(a.text(source), text),
            root,
            a,
            source,
            module,
            text,
            fresh,
        }))
    }

    /// The offset in the text of the check of a position in the document,
    /// when the text around it has not changed since.
    fn old_offset(place: &Place, pos: lsp::Position) -> Option<u32> {
        let offset = LineIndex::new(place.text).offset(pos);
        let word = change::word_at(place.text, offset);
        let old = place.change.old_span(word)?;
        Some(old.start + (offset - word.start))
    }

    /// The range in the document now of a span in the text of a file in a
    /// check, when it has not changed since.
    fn range_now(
        &self,
        a: &Analysis,
        source: SourceId,
        span: ilang_syntax::span::Span,
    ) -> Option<lsp::Range> {
        let old = a.text(source);
        let now = match a.paths.get(&source).and_then(|p| self.docs.get(p)) {
            Some(d) => d.text.as_str(),
            None => old,
        };
        let span = Change::new(old, now).new_span(span)?;
        Some(LineIndex::new(now).range(span))
    }

    fn hover(&self, p: lsp::TextDocumentPositionParams) -> Result<Option<lsp::Hover>, Later> {
        let Some(place) = self.place(&p.text_document.uri)? else {
            return Ok(None);
        };
        let Some(offset) = Self::old_offset(&place, p.position) else {
            return place.later();
        };
        let Some(h) = place
            .a
            .query()
            .and_then(|q| q.type_at(place.module, offset))
        else {
            return Ok(None);
        };
        let Some(range) = self.range_now(place.a, place.source, h.span) else {
            return place.later();
        };
        let mut value = format!("```i\n{}\n```", h.code);
        if let Some(doc) = h.doc {
            value.push_str("\n\n");
            value.push_str(&doc);
        }
        Ok(Some(lsp::Hover {
            contents: lsp::HoverContents::Markup(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value,
            }),
            range: Some(range),
        }))
    }

    fn definition(
        &self,
        p: lsp::TextDocumentPositionParams,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, Later> {
        let Some(place) = self.place(&p.text_document.uri)? else {
            return Ok(None);
        };
        let Some(offset) = Self::old_offset(&place, p.position) else {
            return place.later();
        };
        let a = place.a;
        let Some(loc) = a
            .query()
            .and_then(|q| q.definition_at(place.module, offset))
        else {
            return Ok(None);
        };
        let Some(path) = a.paths.get(&loc.source) else {
            return Ok(None);
        };
        let Some(range) = self.range_now(a, loc.source, loc.span) else {
            return place.later();
        };
        if self.library_dir().is_some_and(|l| path.starts_with(l)) {
            write_library(a);
        }
        Ok(self
            .uri_of(path)
            .map(|uri| lsp::GotoDefinitionResponse::Scalar(lsp::Location { uri, range })))
    }

    fn symbols(
        &self,
        p: lsp::DocumentSymbolParams,
    ) -> Result<Option<lsp::DocumentSymbolResponse>, Later> {
        let Some(place) = self.place(&p.text_document.uri)? else {
            return Ok(None);
        };
        if !place.change.is_same() {
            return place.later();
        }
        let Some(q) = place.a.query() else {
            return Ok(None);
        };
        let ix = LineIndex::new(place.text);
        let symbols = q.symbols(place.module);
        Ok(Some(lsp::DocumentSymbolResponse::Nested(
            symbols.iter().map(|s| document_symbol(&ix, s)).collect(),
        )))
    }

    fn completion(
        &self,
        id: &RequestId,
        p: lsp::TextDocumentPositionParams,
    ) -> Result<Option<lsp::CompletionResponse>, Later> {
        let Some(place) = self.place(&p.text_document.uri)? else {
            return Ok(None);
        };
        let offset = LineIndex::new(place.text).offset(p.position);
        if member_without_name(place.text, offset) {
            // After `v.`, the check before the dot was written, or of the
            // text with the dot, may know `v`.
            let base = change::name_before(place.text, offset - 1);
            if base.start < base.end
                && let Some(old) = place.change.old_name(base, place.old_text())
                && let Some(q) = place.a.query()
            {
                let items = q.member_completions_of(place.module, old);
                if !items.is_empty() {
                    return Ok(Some(completion_response(items)));
                }
            }
            // Else `v.` with nothing after the dot does not parse: check
            // the text with a name after the dot.
            let Some(path) = place.a.paths.get(&place.source).cloned() else {
                return Ok(None);
            };
            let mut text = place.text.to_string();
            text.insert_str(offset as usize, PLACEHOLDER);
            let patch = Patch {
                path,
                text,
                offset,
                request: id.clone(),
            };
            return Err(Later::Check(place.root.clone(), patch));
        }
        if !place.change.is_same() {
            return place.later();
        }
        let Some(q) = place.a.query() else {
            return Ok(None);
        };
        Ok(Some(completion_response(
            q.completions_at(place.module, offset),
        )))
    }

    fn formatting(&mut self, id: RequestId, p: lsp::DocumentFormattingParams) -> Response {
        let Some(format) = format::formatter() else {
            return error(
                id,
                ErrorCode::RequestFailed,
                "formatting is not available in this build of `i`".to_string(),
            );
        };
        let Some(path) = uri::to_path(&p.text_document.uri).map(|p| normalize(&p)) else {
            return Response::new_ok(id, None::<Vec<lsp::TextEdit>>);
        };
        let text = match self.docs.get(&path) {
            Some(d) => d.text.clone(),
            None => std::fs::read_to_string(&path).unwrap_or_default(),
        };
        // `None`: the text has a syntax error, and is left as it is.
        let formatted = if path.extension().is_some_and(|e| e == "is") {
            ilang_syntax::lisp::format(&text).ok()
        } else {
            format(&text)
        };
        let edits = formatted.filter(|f| *f != text).map(|formatted| {
            let ix = LineIndex::new(&text);
            vec![lsp::TextEdit {
                range: lsp::Range::new(lsp::Position::new(0, 0), ix.position(text.len() as u32)),
                new_text: formatted,
            }]
        });
        Response::new_ok(id, edits)
    }
}

/// Writes the files of the library package `i`, so that a definition there
/// can be shown. They are in a private directory of the server.
fn write_library(a: &Analysis) {
    for m in &a.graph.packages[a.graph.library].modules {
        let Some(path) = a.paths.get(&m.source) else {
            continue;
        };
        let text = a.text(m.source);
        if std::fs::read_to_string(path).ok().as_deref() == Some(text) {
            continue;
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, text);
    }
}

/// The name put after `v.` to check the text while completing.
const PLACEHOLDER: &str = "x";

/// `v.` just before `offset`, with no letters of a name after the dot:
/// after a name, or after the end of any other expression, as in `f(x).`
/// or `{ t }.`.
fn member_without_name(text: &str, offset: u32) -> bool {
    let b = text.as_bytes();
    let o = offset as usize;
    o > 1
        && o <= b.len()
        && b[o - 1] == b'.'
        && (b[o - 2].is_ascii_alphanumeric()
            || matches!(b[o - 2], b'_' | b')' | b']' | b'}' | b'"'))
}

/// The parameters of a request, and its answer: `Ok(Err(_))` is an error
/// response.
fn params<P: serde::de::DeserializeOwned, R: serde::Serialize>(
    req: &Request,
    f: impl FnOnce(P) -> Result<Option<R>, Later>,
) -> Result<Result<serde_json::Value, Response>, Later> {
    let p: P = match serde_json::from_value(req.params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return Ok(Err(error(
                req.id.clone(),
                ErrorCode::InvalidParams,
                e.to_string(),
            )));
        }
    };
    let r = f(p)?;
    Ok(serde_json::to_value(r)
        .map_err(|e| error(req.id.clone(), ErrorCode::InternalError, e.to_string())))
}

fn ok<R: serde::Serialize>(id: RequestId, result: R) -> Response {
    Response::new_ok(id, result)
}

fn error(id: RequestId, code: ErrorCode, message: String) -> Response {
    Response::new_err(id, code as i32, message)
}

fn completion_response(items: Vec<query::Completion>) -> lsp::CompletionResponse {
    lsp::CompletionResponse::Array(
        items
            .into_iter()
            .map(|c| lsp::CompletionItem {
                kind: Some(completion_kind(c.kind)),
                detail: c.detail,
                label: c.label,
                ..Default::default()
            })
            .collect(),
    )
}

fn completion_kind(k: CompletionKind) -> lsp::CompletionItemKind {
    match k {
        CompletionKind::Local => lsp::CompletionItemKind::VARIABLE,
        CompletionKind::Function => lsp::CompletionItemKind::FUNCTION,
        CompletionKind::Value => lsp::CompletionItemKind::CONSTANT,
        CompletionKind::Constructor => lsp::CompletionItemKind::CONSTRUCTOR,
        CompletionKind::Type => lsp::CompletionItemKind::CLASS,
        CompletionKind::Module => lsp::CompletionItemKind::MODULE,
        CompletionKind::Field => lsp::CompletionItemKind::FIELD,
        CompletionKind::Keyword => lsp::CompletionItemKind::KEYWORD,
    }
}

fn document_symbol(ix: &LineIndex, s: &query::Symbol) -> lsp::DocumentSymbol {
    #[allow(deprecated)]
    lsp::DocumentSymbol {
        name: s.name.clone(),
        detail: s.detail.clone(),
        kind: match s.kind {
            SymbolKind::Function => lsp::SymbolKind::FUNCTION,
            SymbolKind::Value => lsp::SymbolKind::CONSTANT,
            SymbolKind::Type => lsp::SymbolKind::CLASS,
            SymbolKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
        },
        tags: None,
        deprecated: None,
        range: ix.range(s.span),
        selection_range: ix.range(s.name_span),
        children: (!s.children.is_empty())
            .then(|| s.children.iter().map(|c| document_symbol(ix, c)).collect()),
    }
}
