//! The server: the documents that are open, the packages they are in, and
//! the answers to requests.
//!
//! A package is checked as a whole (`ilang_check::query::check_for_queries`),
//! with the text of the open documents in place of the files on disk. A
//! change marks the packages that have the file as changed; they are
//! checked again when no message has come for `Options::debounce`, or at
//! once when a request needs them.

use crate::position::LineIndex;
use crate::{format, uri};
use crossbeam_channel::RecvTimeoutError;
use ilang_check::Checked;
use ilang_check::hir::ModuleId;
use ilang_check::query::{self, CompletionKind, Query, SymbolKind};
use ilang_package::{Diagnostic, LoadOptions, ModuleKind, PackageGraph, Severity, SourceId};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types as lsp;
use lsp_types::notification::Notification as _;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

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

/// A checked package.
struct Analysis {
    graph: PackageGraph,
    checked: Checked,
    /// The problems found while loading the packages.
    load_diags: Vec<Diagnostic>,
    /// The file of each source, where there is one.
    paths: HashMap<SourceId, PathBuf>,
}

impl Analysis {
    fn source_of(&self, path: &Path) -> Option<SourceId> {
        self.paths
            .iter()
            .find(|(_, p)| p.as_path() == path)
            .map(|(s, _)| *s)
    }

    fn query(&self) -> Option<Query<'_>> {
        let program = self.checked.program.as_ref()?;
        Some(Query::new(
            &self.graph,
            program,
            self.checked.types.as_ref(),
        ))
    }
}

/// A position in a module of a checked package.
struct Place {
    root: PathBuf,
    module: ModuleId,
    offset: u32,
}

struct Server<'c> {
    conn: &'c Connection,
    options: Options,
    /// The open documents, by path.
    docs: HashMap<PathBuf, Doc>,
    /// The checked packages, by the directory of their `i.toml`.
    analyses: HashMap<PathBuf, Analysis>,
    /// The packages to check again.
    dirty: HashSet<PathBuf>,
    /// The URIs with diagnostics, by package.
    published: HashMap<PathBuf, Vec<lsp::Uri>>,
    /// The packages that have been checked once.
    loaded: HashSet<PathBuf>,
    /// Where the files of the library package `i` are written, when a
    /// definition is there.
    library_dir: PathBuf,
}

fn normalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
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
        Server {
            conn,
            options,
            docs: HashMap::new(),
            analyses: HashMap::new(),
            dirty: HashSet::new(),
            published: HashMap::new(),
            loaded: HashSet::new(),
            library_dir: std::env::temp_dir()
                .join(format!("ilang-lsp-{}-library", env!("CARGO_PKG_VERSION"))),
        }
    }

    fn main_loop(&mut self) -> anyhow::Result<()> {
        loop {
            let msg = if self.dirty.is_empty() {
                match self.conn.receiver.recv() {
                    Ok(m) => m,
                    Err(_) => return Ok(()),
                }
            } else {
                match self.conn.receiver.recv_timeout(self.options.debounce) {
                    Ok(m) => m,
                    Err(RecvTimeoutError::Timeout) => {
                        self.check_dirty();
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => return Ok(()),
                }
            };
            match msg {
                Message::Request(req) => {
                    if self.conn.handle_shutdown(&req)? {
                        return Ok(());
                    }
                    let response = self.request(req);
                    self.send(Message::Response(response));
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

    fn log(&self, message: String) {
        self.notify::<lsp::notification::LogMessage>(lsp::LogMessageParams {
            typ: lsp::MessageType::ERROR,
            message,
        });
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
                            self.check(&root);
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
                    self.check_dirty();
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
                    self.dirty.extend(self.analyses.keys().cloned());
                }
            }
            _ => {}
        }
    }

    /// Marks the packages with this file as changed: its own package, and
    /// the checked packages that use it.
    fn changed(&mut self, path: &Path) {
        if let Some(root) = package_root(path) {
            self.dirty.insert(root);
        }
        for (root, a) in &self.analyses {
            if a.paths.values().any(|p| p == path) || path.ends_with("i.toml") {
                self.dirty.insert(root.clone());
            }
        }
    }

    // ---- Checking ----

    fn check_dirty(&mut self) {
        let roots: Vec<PathBuf> = self.dirty.drain().collect();
        for root in roots {
            self.check(&root);
        }
    }

    /// Checks a package again and publishes its problems.
    fn check(&mut self, root: &Path) {
        self.dirty.remove(root);
        let a = self.analyze(root, None);
        self.loaded.insert(root.to_path_buf());
        self.publish(root, &a);
        self.analyses.insert(root.to_path_buf(), a);
    }

    /// Loads and checks the package in `root`, with the text of the open
    /// documents, and `patch` in place of the text of one file.
    fn analyze(&self, root: &Path, patch: Option<(&Path, &str)>) -> Analysis {
        let env = LoadOptions::from_env();
        let options = LoadOptions {
            tests: true,
            offline: env.offline || self.options.offline || self.loaded.contains(root),
            write_lock: false,
            ..env
        };
        let (mut graph, load_diags) = ilang_package::load_with(root, &options);
        let mut paths = HashMap::new();
        for (id, p) in graph.packages.iter().enumerate() {
            let dir = match &p.dir {
                Some(dir) => dir.clone(),
                None if id == graph.library => self.library_dir.clone(),
                None => continue,
            };
            if let Some(s) = p.manifest_source {
                paths.insert(s, dir.join("i.toml"));
            }
            for m in &p.modules {
                let rest = m.path.split_once('/').map_or(m.path.as_str(), |x| x.1);
                let file = format!("{rest}.i");
                let path = match m.kind {
                    ModuleKind::Src => dir.join("src").join(file),
                    ModuleKind::Test => dir.join("test").join(file),
                    ModuleKind::Prelude => dir.join(file),
                };
                paths.insert(m.source, path);
            }
        }
        for (id, path) in &paths {
            if let Some((p, text)) = patch
                && p == path
            {
                graph.sources.set_text(*id, text.to_string());
            } else if let Some(doc) = self.docs.get(path) {
                graph.sources.set_text(*id, doc.text.clone());
            }
        }
        let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            query::check_for_queries(&graph)
        }))
        .unwrap_or_else(|_| {
            self.log(format!(
                "internal error: checking `{}` failed",
                root.display()
            ));
            Checked {
                program: None,
                types: None,
                diags: Vec::new(),
            }
        });
        Analysis {
            graph,
            checked,
            load_diags,
            paths,
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
            if d.source.is_some_and(|s| library_sources.contains(&s)) {
                continue;
            }
            let (path, range) = match d.source.and_then(|s| a.paths.get(&s).map(|p| (s, p))) {
                Some((s, p)) => {
                    let text = &a.graph.sources.get(s).text;
                    (p.clone(), LineIndex::new(text).range(d.span))
                }
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

    fn request(&mut self, req: Request) -> Response {
        use lsp::request::*;
        let id = req.id.clone();
        let result = match req.method.as_str() {
            HoverRequest::METHOD => params(req, |p: lsp::HoverParams| {
                self.hover(p.text_document_position_params)
            }),
            GotoDefinition::METHOD => params(req, |p: lsp::GotoDefinitionParams| {
                self.definition(p.text_document_position_params)
            }),
            DocumentSymbolRequest::METHOD => {
                params(req, |p: lsp::DocumentSymbolParams| self.symbols(p))
            }
            Completion::METHOD => params(req, |p: lsp::CompletionParams| {
                self.completion(p.text_document_position)
            }),
            Formatting::METHOD => {
                return match req.extract::<lsp::DocumentFormattingParams>(Formatting::METHOD) {
                    Ok((id, p)) => self.formatting(id, p),
                    Err(e) => error(id, ErrorCode::InvalidParams, e.to_string()),
                };
            }
            _ => {
                return error(
                    id,
                    ErrorCode::MethodNotFound,
                    format!("unknown request `{}`", req.method),
                );
            }
        };
        match result {
            Ok(value) => Response::new_ok(id, value),
            Err(e) => error(id, ErrorCode::InvalidParams, e),
        }
    }

    /// The checked package and the module of a document, with the offset
    /// of a position. The package is checked first when it has changed.
    fn place(&mut self, uri: &lsp::Uri, pos: lsp::Position) -> Option<Place> {
        let path = normalize(&uri::to_path(uri)?);
        let root = package_root(&path)?;
        if self.dirty.contains(&root) || !self.analyses.contains_key(&root) {
            self.check(&root);
        }
        let a = self.analyses.get(&root)?;
        let source = a.source_of(&path)?;
        let module = a.query()?.module_of_source(source)?;
        let offset = LineIndex::new(&a.graph.sources.get(source).text).offset(pos);
        Some(Place {
            root,
            module,
            offset,
        })
    }

    fn hover(&mut self, p: lsp::TextDocumentPositionParams) -> Option<lsp::Hover> {
        let place = self.place(&p.text_document.uri, p.position)?;
        let a = &self.analyses[&place.root];
        let h = a.query()?.type_at(place.module, place.offset)?;
        let mut value = format!("```i\n{}\n```", h.code);
        if let Some(doc) = h.doc {
            value.push_str("\n\n");
            value.push_str(&doc);
        }
        let text = &a.graph.sources.get(a.query()?.source_of(place.module)).text;
        Some(lsp::Hover {
            contents: lsp::HoverContents::Markup(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value,
            }),
            range: Some(LineIndex::new(text).range(h.span)),
        })
    }

    fn definition(
        &mut self,
        p: lsp::TextDocumentPositionParams,
    ) -> Option<lsp::GotoDefinitionResponse> {
        let place = self.place(&p.text_document.uri, p.position)?;
        let a = &self.analyses[&place.root];
        let loc = a.query()?.definition_at(place.module, place.offset)?;
        let path = a.paths.get(&loc.source)?;
        let text = &a.graph.sources.get(loc.source).text;
        if path.starts_with(&self.library_dir) {
            self.write_library(a);
        }
        Some(lsp::GotoDefinitionResponse::Scalar(lsp::Location {
            uri: self.uri_of(path)?,
            range: LineIndex::new(text).range(loc.span),
        }))
    }

    /// Writes the files of the library package `i`, so that a definition
    /// there can be shown.
    fn write_library(&self, a: &Analysis) {
        for m in &a.graph.packages[a.graph.library].modules {
            let Some(path) = a.paths.get(&m.source) else {
                continue;
            };
            let text = &a.graph.sources.get(m.source).text;
            if std::fs::read_to_string(path).ok().as_deref() == Some(text) {
                continue;
            }
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(path, text);
        }
    }

    fn symbols(&mut self, p: lsp::DocumentSymbolParams) -> Option<lsp::DocumentSymbolResponse> {
        let place = self.place(&p.text_document.uri, lsp::Position::default())?;
        let a = &self.analyses[&place.root];
        let q = a.query()?;
        let text = &a.graph.sources.get(q.source_of(place.module)).text;
        let ix = LineIndex::new(text);
        let symbols = q.symbols(place.module);
        Some(lsp::DocumentSymbolResponse::Nested(
            symbols.iter().map(|s| document_symbol(&ix, s)).collect(),
        ))
    }

    fn completion(
        &mut self,
        p: lsp::TextDocumentPositionParams,
    ) -> Option<lsp::CompletionResponse> {
        let place = self.place(&p.text_document.uri, p.position)?;
        let a = &self.analyses[&place.root];
        let q = a.query()?;
        let source = q.source_of(place.module);
        let text = &a.graph.sources.get(source).text;
        let path = a.paths.get(&source)?.clone();
        let items = match member_without_name(text, place.offset) {
            // `v.` with nothing after the dot does not parse: check the text
            // with a name after the dot.
            true => {
                let mut patched = text.clone();
                patched.insert_str(place.offset as usize, PLACEHOLDER);
                let b = self.analyze(&place.root, Some((&path, &patched)));
                let q = b.query()?;
                let module = q.module_of_source(b.source_of(&path)?)?;
                q.completions_at(module, place.offset)
            }
            false => q.completions_at(place.module, place.offset),
        };
        Some(lsp::CompletionResponse::Array(
            items
                .into_iter()
                .map(|c| lsp::CompletionItem {
                    kind: Some(completion_kind(c.kind)),
                    detail: c.detail,
                    label: c.label,
                    ..Default::default()
                })
                .collect(),
        ))
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
        let edits = format(&text).filter(|f| *f != text).map(|formatted| {
            let ix = LineIndex::new(&text);
            vec![lsp::TextEdit {
                range: lsp::Range::new(lsp::Position::new(0, 0), ix.position(text.len() as u32)),
                new_text: formatted,
            }]
        });
        Response::new_ok(id, edits)
    }
}

/// The name put after `v.` to check the text while completing.
const PLACEHOLDER: &str = "x";

/// `v.` just before `offset`, with no letters of a name after the dot.
fn member_without_name(text: &str, offset: u32) -> bool {
    let b = text.as_bytes();
    let o = offset as usize;
    o > 1
        && o <= b.len()
        && b[o - 1] == b'.'
        && (b[o - 2].is_ascii_alphanumeric() || b[o - 2] == b'_')
}

fn params<P: serde::de::DeserializeOwned, R: serde::Serialize>(
    req: Request,
    f: impl FnOnce(P) -> Option<R>,
) -> Result<serde_json::Value, String> {
    let p: P = serde_json::from_value(req.params).map_err(|e| e.to_string())?;
    serde_json::to_value(f(p)).map_err(|e| e.to_string())
}

fn error(id: RequestId, code: ErrorCode, message: String) -> Response {
    Response::new_err(id, code as i32, message)
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
