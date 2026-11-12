//! The server over an in-memory connection: the messages of an editor
//! session, and the answers.

use ilang_lsp::{Options, uri};
use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types as lsp;
use lsp_types::notification::Notification as _;
use lsp_types::request::Request as _;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

struct Client {
    conn: Connection,
    server: Option<JoinHandle<()>>,
    next_id: i32,
    /// Notifications received while waiting for something else.
    pending: Vec<Notification>,
    /// The `/proc` directory of the server's thread, which answers the
    /// requests: its CPU time is the work of a request, whatever else the
    /// machine runs.
    server_thread: Option<std::path::PathBuf>,
}

const TIMEOUT: Duration = Duration::from_secs(30);

impl Client {
    fn start() -> Client {
        Client::with_debounce(Duration::from_millis(20))
    }

    fn with_debounce(debounce: Duration) -> Client {
        let (server, client) = Connection::memory();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            tx.send(Path::new("/proc").join(fs::read_link("/proc/thread-self").unwrap()))
                .unwrap();
            let options = Options {
                debounce,
                offline: true,
            };
            ilang_lsp::run(&server, options).unwrap();
        });
        let mut c = Client {
            conn: client,
            server: Some(handle),
            next_id: 0,
            pending: Vec::new(),
            server_thread: Some(rx.recv().unwrap()),
        };
        let init = c.request::<lsp::request::Initialize>(json!({ "capabilities": {} }));
        let caps = &init["capabilities"];
        assert_eq!(caps["hoverProvider"], json!(true));
        assert_eq!(caps["definitionProvider"], json!(true));
        assert_eq!(caps["documentSymbolProvider"], json!(true));
        assert_eq!(
            caps["completionProvider"]["triggerCharacters"],
            json!(["."])
        );
        assert_eq!(caps["textDocumentSync"]["change"], json!(1));
        c.notify::<lsp::notification::Initialized>(json!({}));
        c
    }

    /// The CPU time that the server's thread has used.
    fn server_cpu(&self) -> Duration {
        let dir = self.server_thread.as_ref().expect("a server thread");
        let stat = fs::read_to_string(dir.join("schedstat")).unwrap();
        let ns: u64 = stat.split_whitespace().next().unwrap().parse().unwrap();
        Duration::from_nanos(ns)
    }

    fn request<R: lsp::request::Request>(&mut self, params: Value) -> Value {
        self.try_request::<R>(params).unwrap()
    }

    fn try_request<R: lsp::request::Request>(&mut self, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        self.conn
            .sender
            .send(Message::Request(Request::new(
                id.clone(),
                R::METHOD.to_string(),
                params,
            )))
            .unwrap();
        loop {
            match self
                .conn
                .receiver
                .recv_timeout(TIMEOUT)
                .expect("no response")
            {
                Message::Response(r) if r.id == id => {
                    return r.response_result.map_err(|e| e.message);
                }
                Message::Notification(n) => self.pending.push(n),
                _ => {}
            }
        }
    }

    fn notify<N: lsp::notification::Notification>(&mut self, params: Value) {
        self.conn
            .sender
            .send(Message::Notification(Notification::new(
                N::METHOD.to_string(),
                params,
            )))
            .unwrap();
    }

    /// The next diagnostics published for `uri`.
    fn diagnostics(&mut self, uri: &str) -> Vec<lsp::Diagnostic> {
        self.diagnostics_within(uri, TIMEOUT)
    }

    fn diagnostics_within(&mut self, uri: &str, timeout: Duration) -> Vec<lsp::Diagnostic> {
        let is_it = |n: &Notification| {
            n.method == lsp::notification::PublishDiagnostics::METHOD && n.params["uri"] == uri
        };
        let n = match self.pending.iter().position(is_it) {
            Some(i) => self.pending.remove(i),
            None => loop {
                match self
                    .conn
                    .receiver
                    .recv_timeout(timeout)
                    .expect("no diagnostics")
                {
                    Message::Notification(n) if is_it(&n) => break n,
                    Message::Notification(n) => self.pending.push(n),
                    _ => {}
                }
            },
        };
        let p: lsp::PublishDiagnosticsParams = serde_json::from_value(n.params).unwrap();
        p.diagnostics
    }

    /// Diagnostics for `uri` have come and have not been read, without
    /// waiting for any.
    fn has_diagnostics(&mut self, uri: &str) -> bool {
        while let Ok(m) = self.conn.receiver.try_recv() {
            if let Message::Notification(n) = m {
                self.pending.push(n);
            }
        }
        self.pending.iter().any(|n| {
            n.method == lsp::notification::PublishDiagnostics::METHOD && n.params["uri"] == uri
        })
    }

    fn open(&mut self, uri: &str, text: &str) {
        self.notify::<lsp::notification::DidOpenTextDocument>(json!({
            "textDocument": { "uri": uri, "languageId": "i", "version": 1, "text": text }
        }));
    }

    fn change(&mut self, uri: &str, version: i32, text: &str) {
        self.notify::<lsp::notification::DidChangeTextDocument>(json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [{ "text": text }]
        }));
    }

    fn shutdown(mut self) {
        let r = self.request::<lsp::request::Shutdown>(Value::Null);
        assert_eq!(r, Value::Null);
        self.notify::<lsp::notification::Exit>(Value::Null);
        self.server.take().unwrap().join().unwrap();
    }
}

fn position_params(uri: &str, line: u32, character: u32) -> Value {
    json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": character } })
}

/// The line and UTF-16 character of the `nth` occurrence of `needle`.
fn find(text: &str, needle: &str, nth: usize) -> (u32, u32) {
    let (i, _) = text
        .match_indices(needle)
        .nth(nth)
        .unwrap_or_else(|| panic!("no {needle:?} #{nth} in\n{text}"));
    let before = &text[..i];
    let line = before.matches('\n').count() as u32;
    let col = before[before.rfind('\n').map_or(0, |n| n + 1)..]
        .encode_utf16()
        .count() as u32;
    (line, col)
}

/// The exact LSP diagnostic for an unused top-level binding.
fn unused_diagnostic(text: &str, name: &str, binding: &str) -> lsp::Diagnostic {
    let (line, col) = find(text, binding, 0);
    lsp::Diagnostic {
        range: lsp::Range::new(
            lsp::Position::new(line, col),
            lsp::Position::new(line, col + name.encode_utf16().count() as u32),
        ),
        severity: Some(lsp::DiagnosticSeverity::WARNING),
        code: Some(lsp::NumberOrString::String("W0504".into())),
        source: Some("i".into()),
        message: format!("top-level binding `{name}` is not used"),
        tags: Some(vec![lsp::DiagnosticTag::UNNECESSARY]),
        data: Some(json!({ "fix": "remove" })),
        ..Default::default()
    }
}

fn unused_diagnostics(text: &str, names: &[&str]) -> Vec<lsp::Diagnostic> {
    names
        .iter()
        .map(|name| unused_diagnostic(text, name, &format!("{name} =")))
        .collect()
}

fn package(dir: &Path, files: &[(&str, &str)]) {
    fs::write(dir.join("i.toml"), "name = \"app\"\n").unwrap();
    for (path, text) in files {
        let p = dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
}

fn file_uri(path: &Path) -> String {
    uri::from_path(&path.canonicalize().unwrap())
        .unwrap()
        .as_str()
        .to_string()
}

const MAIN: &str = "(\n\tutil = app/util\n)\n\n/// Adds two numbers.\nadd = (x, y) { x + y }\n\nUser = (name = String, age = Int)\n\ngreet = (u: User) { \"héllo 𝕏 \" <> u.name }\n\nmain = () { util.double(add(1, 2)) }\n";

const UTIL: &str = " (double)\n\ndouble = (n: Int) -> Int { n * 2 }\n";

#[test]
fn an_editor_session() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", MAIN), ("src/util.i", UTIL)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let util = file_uri(&dir.path().join("src/util.i"));
    let mut c = Client::start();

    // Opening a correct file clears its problems.
    c.open(&main, MAIN);
    assert_eq!(c.diagnostics(&main), unused_diagnostics(MAIN, &["greet"]));

    // Hover shows the type, with the documentation.
    let (l, ch) = find(MAIN, "add(1", 0);
    let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch + 1));
    let value = h["contents"]["value"].as_str().unwrap();
    assert!(value.contains("add: (x = Int, y = Int) -> Int"), "{value}");
    assert!(value.contains("Adds two numbers."), "{value}");
    assert_eq!(h["range"]["start"], json!({ "line": l, "character": ch }));

    // After characters that take 2 and 4 bytes, and 1 and 2 UTF-16 units.
    let (l, ch) = find(MAIN, "u.name", 0);
    let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch));
    assert_eq!(h["contents"]["value"], "```i\nu: User\n```");
    assert_eq!(h["range"]["start"], json!({ "line": l, "character": ch }));

    // Go to definition, in this file and in another.
    let (l, ch) = find(MAIN, "add(1", 0);
    let d = c.request::<lsp::request::GotoDefinition>(position_params(&main, l, ch));
    let (dl, dch) = find(MAIN, "add =", 0);
    assert_eq!(d["uri"], main);
    assert_eq!(d["range"]["start"], json!({ "line": dl, "character": dch }));
    let (l, ch) = find(MAIN, "double", 0);
    let d = c.request::<lsp::request::GotoDefinition>(position_params(&main, l, ch + 2));
    assert_eq!(d["uri"], util);
    let (dl, dch) = find(UTIL, "double =", 0);
    assert_eq!(d["range"]["start"], json!({ "line": dl, "character": dch }));

    // A type.
    let (l, ch) = find(MAIN, "User)", 0);
    let d = c.request::<lsp::request::GotoDefinition>(position_params(&main, l, ch));
    assert_eq!(d["uri"], main);
    let (dl, dch) = find(MAIN, "User =", 0);
    assert_eq!(d["range"]["start"], json!({ "line": dl, "character": dch }));

    // Document symbols.
    let s = c
        .request::<lsp::request::DocumentSymbolRequest>(json!({ "textDocument": { "uri": main } }));
    let names: Vec<&str> = s
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["add", "User", "greet", "main"]);
    assert_eq!(s[1]["children"][0]["name"], "User");

    // Completion of the names in scope.
    let (l, ch) = find(MAIN, "x + y", 0);
    let r = c.request::<lsp::request::Completion>(position_params(&main, l, ch));
    let labels: Vec<&str> = r
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["label"].as_str().unwrap())
        .collect();
    for name in ["x", "y", "add", "greet", "util", "User", "Int", "panic"] {
        assert!(labels.contains(&name), "{name} in {labels:?}");
    }

    assert!(!labels.contains(&"case"));

    // A change with an error: the problem is published, with its code.
    let broken = MAIN.replace("x + y", "x + z");
    c.change(&main, 2, &broken);
    let diags = c.diagnostics(&main);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(
        diags[0].code,
        Some(lsp::NumberOrString::String("E0201".to_string()))
    );
    assert_eq!(diags[0].severity, Some(lsp::DiagnosticSeverity::ERROR));
    let (l, ch) = find(&broken, "z }", 0);
    assert_eq!(diags[0].range.start, lsp::Position::new(l, ch));
    assert_eq!(diags[0].range.end, lsp::Position::new(l, ch + 1));

    // A type error.
    let broken = MAIN.replace("x + y", "x + \"y\"");
    c.change(&main, 3, &broken);
    let diags = c.diagnostics(&main);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(lsp::NumberOrString::String("E0501".into()))),
        "{diags:?}"
    );

    // Fixed again: the problems are cleared.
    c.change(&main, 4, MAIN);
    assert_eq!(c.diagnostics(&main), unused_diagnostics(MAIN, &["greet"]));

    // Completion after `u.` in an unsaved buffer that does not parse.
    let typing = MAIN.replace("u.name", "u.");
    c.change(&main, 5, &typing);
    let (l, ch) = find(&typing, "u.", 0);
    let r = c.request::<lsp::request::Completion>(position_params(&main, l, ch + 2));
    let labels: Vec<&str> = r
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, vec!["name", "age"]);
    // And after `util.`.
    let typing = MAIN.replace("util.double", "util.");
    c.change(&main, 6, &typing);
    let (l, ch) = find(&typing, "util.", 0);
    let r = c.request::<lsp::request::Completion>(position_params(&main, l, ch + 5));
    assert_eq!(r[0]["label"], "double");
    assert_eq!(r[0]["detail"], "(n = Int) -> Int");

    // Formatting gives the text of `i fmt` as one edit of the whole document.
    c.change(&main, 7, MAIN);
    let edits = c
        .try_request::<lsp::request::Formatting>(json!({
            "textDocument": { "uri": main },
            "options": { "tabSize": 4, "insertSpaces": false }
        }))
        .unwrap();
    let expected = ilang_syntax::format::format(MAIN).unwrap();
    if expected == MAIN {
        assert!(
            edits.is_null()
                || edits.as_array().is_some_and(|e| e.is_empty())
                || edits[0]["newText"] == expected
        );
    } else {
        assert_eq!(edits[0]["newText"], expected);
    }

    c.shutdown();
}

#[test]
fn unsaved_changes_in_another_file() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", MAIN), ("src/util.i", UTIL)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let util = file_uri(&dir.path().join("src/util.i"));
    let mut c = Client::start();
    c.open(&main, MAIN);
    assert_eq!(c.diagnostics(&main), unused_diagnostics(MAIN, &["greet"]));
    // `double` is no longer exported, in the buffer only.
    c.open(&util, &UTIL.replace(" (double)", " "));
    let diags = c.diagnostics(&main);
    assert_eq!(
        diags.first().and_then(|d| d.code.clone()),
        Some(lsp::NumberOrString::String("E0204".into())),
        "{diags:?}"
    );
    // Closing the buffer goes back to the file on disk.
    c.notify::<lsp::notification::DidCloseTextDocument>(json!({
        "textDocument": { "uri": util }
    }));
    assert_eq!(c.diagnostics(&main), unused_diagnostics(MAIN, &["greet"]));
    c.shutdown();
}

#[test]
fn a_file_outside_a_package() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("loose.i");
    fs::write(&path, "x = 1\n").unwrap();
    let loose = file_uri(&path);
    let mut c = Client::start();
    c.open(&loose, "x = 1\n");
    let h = c.request::<lsp::request::HoverRequest>(position_params(&loose, 0, 0));
    assert_eq!(h, Value::Null);
    c.shutdown();
}

#[test]
fn a_definition_in_the_library() {
    let dir = tempfile::tempdir().unwrap();
    let text = "main = () { panic(\"no\") }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    let d = c.request::<lsp::request::GotoDefinition>(position_params(&main, 0, 13));
    let target = d["uri"].as_str().unwrap();
    assert!(target.ends_with("/prelude.i"), "{target}");
    // The file is written, so that the editor can show it.
    let uri: lsp::Uri = target.parse().unwrap();
    let written = fs::read_to_string(uri::to_path(&uri).unwrap()).unwrap();
    let line = d["range"]["start"]["line"].as_u64().unwrap() as usize;
    assert!(written.lines().nth(line).unwrap().starts_with("panic:"));
    c.shutdown();
}

/// The files of the library are written in a new directory that only this
/// user may use, never in a place that another user could know of and put
/// a link in.
#[test]
fn the_library_is_written_in_a_private_directory() {
    let dir = tempfile::tempdir().unwrap();
    let text = "main = () { panic(\"no\") }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    // A link at the place where the library was once written, to a file
    // of the user.
    let victim = dir.path().join("victim.txt");
    fs::write(&victim, "mine\n").unwrap();
    let old = std::env::temp_dir().join(format!("ilang-lsp-{}-library", env!("CARGO_PKG_VERSION")));
    let link = old.join("prelude.i");
    #[cfg(unix)]
    let planted = fs::create_dir_all(&old).is_ok()
        && (fs::symlink_metadata(&link).is_err() || fs::remove_file(&link).is_ok())
        && std::os::unix::fs::symlink(&victim, &link).is_ok();
    #[cfg(not(unix))]
    let planted = false;

    let mut c = Client::start();
    c.open(&main, text);
    let d = c.request::<lsp::request::GotoDefinition>(position_params(&main, 0, 13));
    let uri: lsp::Uri = d["uri"].as_str().unwrap().parse().unwrap();
    let target = uri::to_path(&uri).unwrap();
    let library = target.parent().unwrap().to_path_buf();
    assert!(!target.starts_with(&old), "{}", target.display());
    assert!(fs::read_to_string(&target).unwrap().contains("panic:"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&library).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "{mode:o}");
    }
    assert_eq!(fs::read_to_string(&victim).unwrap(), "mine\n");
    if planted {
        let _ = fs::remove_file(&link);
    }
    // The directory is removed when the server stops.
    c.shutdown();
    assert!(!library.exists(), "{}", library.display());
}

/// An `i.toml` with changes that are not saved: it is loaded from the
/// file, so its problems are only shown when the document is the file.
#[test]
fn an_unsaved_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = "name = \"app\"\n\n[dependencies]\nnope = { path = \"../nowhere\" }\n";
    package(dir.path(), &[("src/main.i", "x = 1\n")]);
    fs::write(dir.path().join("i.toml"), manifest).unwrap();
    let main = file_uri(&dir.path().join("src/main.i"));
    let toml = file_uri(&dir.path().join("i.toml"));
    let mut c = Client::start();
    c.open(&main, "x = 1\n");
    let diags = c.diagnostics(&toml);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].range.start, lsp::Position::new(3, 0));
    // Three lines more in the document: the problem found in the file is
    // not put on the wrong line.
    let edited = format!("# one\n# two\n# three\n{manifest}");
    c.open(&toml, &edited);
    assert_eq!(c.diagnostics(&toml), vec![]);
    // Saved: the problem is shown where it is.
    fs::write(dir.path().join("i.toml"), &edited).unwrap();
    c.notify::<lsp::notification::DidSaveTextDocument>(json!({
        "textDocument": { "uri": toml }
    }));
    let diags = c.diagnostics(&toml);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].range.start, lsp::Position::new(6, 0));
    c.shutdown();
}

/// A new document in `src/` that is not a file yet is a module.
#[test]
fn a_new_document() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", "x = 1\n")]);
    let path = dir.path().canonicalize().unwrap().join("src/new.i");
    let new = uri::from_path(&path).unwrap().as_str().to_string();
    let text = "y = 2\nz = y + \"s\"\n";
    let mut c = Client::start();
    c.open(&new, text);
    let diags = c.diagnostics(&new);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(lsp::NumberOrString::String("E0501".into()))),
        "{diags:?}"
    );
    let h = c.request::<lsp::request::HoverRequest>(position_params(&new, 0, 0));
    assert_eq!(h["contents"]["value"], "```i\ny: 2\n```");
    // Fixed: the problems are cleared.
    c.change(&new, 2, "y = 2\nz = y + 1\n");
    assert_eq!(
        c.diagnostics(&new),
        unused_diagnostics("y = 2\nz = y + 1\n", &["z"])
    );
    c.shutdown();
}

/// Completion after a dot that follows an expression that is not a name.
#[test]
fn completion_after_an_expression() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Point = (x = Int, y = Int)\n\
                make = () -> Point { Point(x = 1, y = 2) }\n\
                f = (t: Point) {\n\
                \ta = make().x\n\
                \tb = { t }.y\n\
                \ta + b\n\
                }\n\
                ";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    assert_eq!(c.diagnostics(&main), unused_diagnostics(text, &["f"]));
    for (version, (from, to)) in [("make().x", "make()."), ("{ t }.y", "{ t }.")]
        .into_iter()
        .enumerate()
    {
        let typing = text.replace(from, to);
        c.change(&main, version as i32 + 2, &typing);
        let (l, ch) = find(&typing, to, 0);
        let r =
            c.request::<lsp::request::Completion>(position_params(&main, l, ch + to.len() as u32));
        let labels: Vec<&str> = r
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["label"].as_str().unwrap())
            .collect();
        assert_eq!(labels, vec!["x", "y"], "after {to}");
    }
    c.shutdown();
}

/// Completion inside a typed macro argument comes from the signature check,
/// also when the macro itself fails to expand.
#[test]
fn typed_macro_argument_completion_without_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let text = "(\n\tm = app/macros\n)\nPoint = (x = Int, y = Int)\nmain = () {\n\tp = Point(x = 1, y = 2)\n\tm.broken(p.x)\n}\n";
    let macros = "(, (= syntax (/ i syntax)) (= fail (/ i fail)))\n(, broken)\n(@ macro (: broken (-> [A] (, A) A)))\n(@ macro (= broken (-> (, value) _ ((. fail raise) ((. syntax SyntaxError) \"never expands\" value)))))\n";
    package(
        dir.path(),
        &[("src/main.i", text), ("src/macros.is", macros)],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    let ds = c.diagnostics(&main);
    assert!(
        ds.iter().any(|d| d.message.contains("never expands")),
        "{ds:?}"
    );
    let typing = text.replace("p.x)", "p.)");
    c.change(&main, 2, &typing);
    let (l, ch) = find(&typing, "p.)", 0);
    let r = c.request::<lsp::request::Completion>(position_params(&main, l, ch + 2));
    assert_eq!(labels(&r), vec!["x", "y"]);
    c.shutdown();
}

fn labels(r: &Value) -> Vec<&str> {
    r.as_array()
        .unwrap()
        .iter()
        .map(|i| i["label"].as_str().unwrap())
        .collect()
}

/// While a package waits to be checked again after a change, hover and go
/// to definition are answered from the last check where the text around
/// the position has not changed; elsewhere they wait for a new check,
/// which starts at once.
#[test]
fn answers_from_the_last_check() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", MAIN), ("src/util.i", UTIL)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    // Long enough that no check starts by itself during the test.
    let mut c = Client::with_debounce(Duration::from_secs(600));
    c.open(&main, MAIN);
    assert_eq!(c.diagnostics(&main), unused_diagnostics(MAIN, &["greet"]));
    let started = Instant::now();
    // A line before everything.
    let edited = format!("\n{MAIN}");
    c.change(&main, 2, &edited);
    let (l, ch) = find(&edited, "add(1", 0);
    let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch + 1));
    let value = h["contents"]["value"].as_str().unwrap();
    assert!(value.contains("add: (x = Int, y = Int) -> Int"), "{value}");
    assert_eq!(h["range"]["start"], json!({ "line": l, "character": ch }));
    let d = c.request::<lsp::request::GotoDefinition>(position_params(&main, l, ch));
    let (dl, dch) = find(&edited, "add =", 0);
    assert_eq!(d["range"]["start"], json!({ "line": dl, "character": dch }));
    // Answered without a check: none has published problems.
    assert!(!c.has_diagnostics(&main));
    assert!(started.elapsed() < Duration::from_secs(300));
    // New bindings at the end. In the new text, hover waits for a check,
    // which publishes the new problem.
    let edited = format!("{edited}extra = add(1, 2)\nbad = nope\n");
    c.change(&main, 3, &edited);
    let (l, ch) = find(&edited, "extra", 0);
    let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch));
    assert_eq!(h["contents"]["value"], "```i\nextra: Int\n```");
    let diags = c.diagnostics(&main);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(
        diags[0].range.start,
        lsp::Position::new(find(&edited, "nope", 0).0, 6)
    );
    c.shutdown();
}

/// Completion after a dot just written is answered at once: from the last
/// check where it knows the name before the dot, else from a check of the
/// text with a name after the dot, without waiting for the package to be
/// checked again.
#[test]
fn completion_after_a_dot_just_written() {
    let dir = tempfile::tempdir().unwrap();
    let text = format!("{MAIN}\nmake = () -> User {{ User(\"a\", 1) }}\nm = make().name\n");
    package(dir.path(), &[("src/main.i", &text), ("src/util.i", UTIL)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::with_debounce(Duration::from_secs(600));
    c.open(&main, &text);
    assert_eq!(
        c.diagnostics(&main),
        unused_diagnostics(&text, &["greet", "m"])
    );
    let started = Instant::now();
    for (version, (from, to)) in [("u.name", "u."), ("make().name", "make().")]
        .into_iter()
        .enumerate()
    {
        let typing = text.replace(from, to);
        c.change(&main, version as i32 + 2, &typing);
        let (l, ch) = find(&typing, to, 0);
        let r =
            c.request::<lsp::request::Completion>(position_params(&main, l, ch + to.len() as u32));
        assert_eq!(labels(&r), vec!["name", "age"], "after {to}");
    }
    assert!(!c.has_diagnostics(&main));
    assert!(started.elapsed() < Duration::from_secs(300));
    c.shutdown();
}

/// Changes that come quickly one after the other are checked once.
#[test]
fn quick_changes_are_checked_once() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", MAIN), ("src/util.i", UTIL)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::with_debounce(Duration::from_millis(300));
    c.open(&main, MAIN);
    assert_eq!(c.diagnostics(&main), unused_diagnostics(MAIN, &["greet"]));
    for i in 0..10 {
        let text = format!("{MAIN}{}bad = nope\n", "\n".repeat(i));
        c.change(&main, i as i32 + 2, &text);
    }
    let diags = c.diagnostics(&main);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].range.start.line, MAIN.lines().count() as u32 + 9);
    std::thread::sleep(Duration::from_millis(900));
    assert!(!c.has_diagnostics(&main));
    c.shutdown();
}

/// Hover on a local name names its type variables as the type scheme of
/// the top-level binding that it is in does.
#[test]
fn hover_on_a_local_name_uses_the_names_of_the_scheme() {
    let text = "k = (f, x) {\n  y = f(x)\n  y\n}\n\
                \n\
                compose = (f, g) {\n  (x) { g(f(x)) }\n}\n\
                \n\
                pairup = (a, b) { (b, a) }\n";
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    assert_eq!(
        c.diagnostics(&main),
        unused_diagnostics(text, &["k", "compose", "pairup"])
    );
    let mut hover = |needle: &str| {
        let (l, ch) = find(text, needle, 0);
        let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch));
        let value = h["contents"]["value"].as_str().unwrap();
        value
            .trim_start_matches("```i\n")
            .trim_end_matches("\n```")
            .to_string()
    };
    assert_eq!(
        hover("k ="),
        "k: (f = (A) <- (Fail(E), ..N) -> B, x = A) <- (Fail(E), ..N) -> B"
    );
    assert_eq!(hover("y ="), "y: B");
    assert_eq!(hover("f, x"), "f: (A) <- (Fail(E), ..N) -> B");
    assert_eq!(hover("g) {"), "g: (B) <- (Fail(E2), ..N2) -> C");
    assert_eq!(hover("f, g"), "f: (A) <- (Fail(E), ..N) -> B");
    assert_eq!(hover("x) { g"), "x: A");
    assert_eq!(hover("b) {"), "b: B");
    assert_eq!(hover("a, b"), "a: A");
    c.shutdown();
}

/// Hover on a local name whose type variable is on one side only of the
/// scheme of its binding, where it is shown as a name of its own, shows it
/// with that name.
#[test]
fn hover_on_a_local_name_names_one_sided_variables_as_the_scheme() {
    let text = "one = (x) {\n  y = x\n  1\n}\n\
                \n\
                drop2 = (x, g, h) {\n  y = (x, h)\n  g(1)\n}\n\
                \n\
                effonly = (f, x) {\n  r = f(x)\n  Nil\n}\n";
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    assert_eq!(
        c.diagnostics(&main),
        unused_diagnostics(text, &["one", "drop2", "effonly"])
    );
    let mut hover = |needle: &str| {
        let (l, ch) = find(text, needle, 0);
        let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch));
        let value = h["contents"]["value"].as_str().unwrap();
        value
            .trim_start_matches("```i\n")
            .trim_end_matches("\n```")
            .to_string()
    };
    assert_eq!(hover("one"), "one: (x = A) -> 1");
    assert_eq!(hover("x) {\n  y = x"), "x: A");
    assert_eq!(hover("y = x"), "y: A");
    assert_eq!(
        hover("drop2"),
        "drop2: (x = A, g = (1) <- (Fail(E), ..N) -> B, h = C) <- (Fail(E), ..N) -> B"
    );
    assert_eq!(hover("x, g, h"), "x: A");
    assert_eq!(hover("h) {"), "h: C");
    assert_eq!(hover("y = (x, h)"), "y: (A, C)");
    assert_eq!(
        hover("effonly"),
        "effonly: (f = (A) <- (Fail(E), ..N) -> B, x = A) <- (Fail(E), ..N) -> Nil"
    );
    assert_eq!(hover("f, x) {\n  r"), "f: (A) <- (Fail(E), ..N) -> B");
    assert_eq!(hover("r = "), "r: B");
    c.shutdown();
}

#[test]
fn t2_nested_pattern_heads_support_hover_and_definition_in_recovery() {
    // Every pattern head targets its existing type, at every depth.
    let source = include_str!("../../ilang-check/tests/value_type_review/nested/src/main.i")
        .replace("x: Item = 2", "x: Item = 1");
    let source = source.as_str();
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", source)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, source);
    assert_eq!(
        c.diagnostics(&main),
        vec![unused_diagnostic(source, "x", "x: Item")]
    );
    let broken = source.replace("= v\n", "= missing\n");
    for (i, text) in [source, &broken].into_iter().enumerate() {
        if i != 0 {
            c.change(&main, 2, text);
            assert!(!c.diagnostics(&main).is_empty());
        }
        let (line, col) = find(text, "Box(Item)", 0);
        let hover = c.request::<lsp::request::HoverRequest>(position_params(&main, line, col));
        assert!(
            hover["contents"]["value"]
                .as_str()
                .unwrap()
                .contains("Box(A)")
        );
        let definition =
            c.request::<lsp::request::GotoDefinition>(position_params(&main, line, col));
        let (line, col) = find(text, "Box(A)", 0);
        assert_eq!(definition["uri"], main);
        assert_eq!(
            definition["range"]["start"],
            json!({"line":line,"character":col})
        );
    }
    c.shutdown();
}

#[test]
fn disjoint_comparison_is_a_warning_at_the_comparison() {
    // W520 keeps the comparison span and shows both types.
    let source = "x = 1 == 2\n";
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", source)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, source);
    let ds = c.diagnostics(&main);
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(ds[0], unused_diagnostics(source, &["x"])[0]);
    let ds = &ds[1..];
    assert_eq!(
        ds[0].code,
        Some(lsp::NumberOrString::String("W0503".into()))
    );
    assert_eq!(ds[0].severity, Some(lsp::DiagnosticSeverity::WARNING));
    assert_eq!(
        ds[0].range,
        lsp::Range::new(lsp::Position::new(0, 4), lsp::Position::new(0, 10))
    );
    assert!(ds[0].message.contains("`1` and `2`"), "{ds:?}");
    assert!(ds[0].message.contains("can never be equal"), "{ds:?}");
}

#[test]
fn macro_expansion_preserves_hover_on_user_argument() {
    let dir = tempfile::tempdir().unwrap();
    let main_text = "(m = app/macros)\nmain = () {\n  x = 42\n  m.pass(x)\n}\n";
    let macro_text = "(, (= syntax (/ i syntax)))\n(, pass)\n(@ macro (= pass (-> (, (: x (. syntax Syntax))) (. syntax Syntax) x)))\n";
    package(
        dir.path(),
        &[("src/main.i", main_text), ("src/macros.is", macro_text)],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut client = Client::start();
    client.open(&main, main_text);
    let diagnostics = client.diagnostics(&main);
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.severity == Some(lsp::DiagnosticSeverity::ERROR)),
        "{diagnostics:?}"
    );
    let (line, character) = find(main_text, "pass(x)", 0);
    let hover =
        client.request::<lsp::request::HoverRequest>(position_params(&main, line, character + 5));
    assert!(
        hover["contents"]["value"]
            .as_str()
            .is_some_and(|value| value.contains("x: 42")),
        "{hover:?}"
    );
}

#[test]
fn semantic_tokens_and_hover_inside_raw_interpolation() {
    let dir = tempfile::tempdir().unwrap();
    let source = "main = () { name = \"i\"  #\"raw \\(name) \\#(name)\"# }\n";
    package(dir.path(), &[("src/main.i", source)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut client = Client::start();
    client.open(&main, source);
    let result = client.request::<lsp::request::SemanticTokensFullRequest>(json!({
        "textDocument": { "uri": main }
    }));
    let data: lsp::SemanticTokens = serde_json::from_value(result).unwrap();
    let (_, character) = find(source, "\\#(name)", 0);
    let mut column = 0;
    assert!(data.data.iter().any(|token| {
        assert_eq!(token.delta_line, 0);
        column += token.delta_start;
        column == character + 3 && token.length == 4 && token.token_type == 3
    }));
    let hover =
        client.request::<lsp::request::HoverRequest>(position_params(&main, 0, character + 3));
    assert!(
        hover["contents"]["value"]
            .as_str()
            .is_some_and(|s| s.contains("name:")),
        "{hover:?}"
    );
}

#[test]
fn typed_macro_hover_and_definition_through_call() {
    let dir = tempfile::tempdir().unwrap();
    let source = "(m = app/macros)\nmain = () {\n  x = 42\n  m.pass(x)\n}\n";
    let macros = "(, (= syntax (/ i syntax)))\n(, pass)\n/// A typed forwarding macro.\n(@ macro (: pass (-> [A] (, A) A)))\n(@ macro (= pass (-> (, value) _ value)))\n";
    package(
        dir.path(),
        &[("src/main.i", source), ("src/macros.is", macros)],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let mac = file_uri(&dir.path().join("src/macros.is"));
    let mut client = Client::start();
    client.open(&main, source);
    let ds = client.diagnostics(&main);
    assert!(
        !ds.iter()
            .any(|d| d.severity == Some(lsp::DiagnosticSeverity::ERROR)),
        "{ds:?}"
    );
    let (line, column) = find(source, "pass(x)", 0);
    let hover =
        client.request::<lsp::request::HoverRequest>(position_params(&main, line, column + 5));
    assert!(
        hover["contents"]["value"]
            .as_str()
            .is_some_and(|s| s.contains("x: 42")),
        "{hover:?}"
    );
    let definition =
        client.request::<lsp::request::GotoDefinition>(position_params(&main, line, column + 1));
    assert_eq!(definition["uri"], mac);
    // The definition is the binding of the macro, as for any name that also has a declaration.
    assert_eq!(definition["range"]["start"]["line"], 4);
}

// ---- Features over a package with modules, tests, and a dependency ----

const LIB_TEXT: &str = "(\n\tshout\n\tTone(..)\n)\n\nLoud = ()\nQuiet = ()\n/// A tone of voice.\nTone = Loud | Quiet\n\n/// Makes a text louder.\nshout = (s: String) -> String { s <> \"!\" }\n";

const SHAPES: &str = "(\n\tShape(..)\n\tarea\n\torigin\n\tClock(..)\n\tBox(..)\n)\n\nCircle = (radius = Int)\nRect = (width = Int, height = Int)\n/// A shape in the plane.\nShape = Circle | Rect\n\n/// A box of a value.\nBox(A) = (item = A)\n\n/// A clock service.\nClock = (now = () -> Int)\n\n/// The area of a shape.\narea = (s: Shape) -> Int {\n\ts |>\n\t\t(Circle(radius = radius)) { radius * radius * 3 }\n\t\t| (Rect(width = width, height = height)) { width * height }\n}\n\n/// The place where shapes start.\norigin: Int = 0\n";

const APP_MAIN: &str = "(\n\tshapes = (Shape, Circle, area) = app/shapes\n\tlib/text\n)\n\n(\n\tmain\n)\n\n/// How big a shape is.\nsize = (s: Shape) -> Int {\n\ta = area(s)\n\ta + shapes.origin\n}\n\nmain = () {\n\tc = Circle(radius = 2)\n\tn = size(c)\n\tt = text.shout(\"hi\")\n\tn\n}\n";

const APP_TEST: &str =
    "(\n\tapp/shapes\n)\n\n_check = () { shapes.area(shapes.Circle(radius = 1)) }\n";

/// A workspace with the package `app`, which depends on the package `lib`
/// next to it. With `lisp`, `src/main` of `app` is in the Lisp surface.
struct Ws {
    _dir: tempfile::TempDir,
    root: std::path::PathBuf,
    main: String,
    main_text: String,
    shapes: String,
    test: String,
    lib: String,
}

fn to_lisp(text: &str) -> String {
    let (module, diags) = ilang_syntax::syn3::parse_module(text);
    assert!(diags.is_empty(), "{diags:?}");
    ilang_syntax::syn3::lisp::to_lisp(&module)
}

fn workspace(lisp: bool) -> Ws {
    workspace_with(lisp, APP_MAIN)
}

fn workspace_with(lisp: bool, main: &str) -> Ws {
    let dir = tempfile::tempdir().unwrap();
    let lib = dir.path().join("lib");
    let app = dir.path().join("app");
    fs::create_dir_all(lib.join("src")).unwrap();
    fs::create_dir_all(app.join("src")).unwrap();
    fs::create_dir_all(app.join("test")).unwrap();
    fs::write(lib.join("i.toml"), "name = \"lib\"\n").unwrap();
    fs::write(lib.join("src/text.i"), LIB_TEXT).unwrap();
    fs::write(
        app.join("i.toml"),
        "name = \"app\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n",
    )
    .unwrap();
    fs::write(app.join("src/shapes.i"), SHAPES).unwrap();
    fs::write(app.join("test/shapes_test.i"), APP_TEST).unwrap();
    let (main_path, main_text) = if lisp {
        (app.join("src/main.is"), to_lisp(main))
    } else {
        (app.join("src/main.i"), main.to_string())
    };
    fs::write(&main_path, &main_text).unwrap();
    Ws {
        main: file_uri(&main_path),
        main_text,
        shapes: file_uri(&app.join("src/shapes.i")),
        test: file_uri(&app.join("test/shapes_test.i")),
        lib: file_uri(&lib.join("src/text.i")),
        root: app.canonicalize().unwrap(),
        _dir: dir,
    }
}

impl Ws {
    /// A client with `src/main` open and checked.
    fn open(&self) -> Client {
        let mut c = Client::start();
        c.open(&self.main, &self.main_text);
        let diags = c.diagnostics(&self.main);
        assert!(
            !diags
                .iter()
                .any(|d| d.severity == Some(lsp::DiagnosticSeverity::ERROR)),
            "{diags:?}\n{}",
            self.main_text
        );
        c
    }
}

/// The position of the `nth` `needle` in `text`, `delta` characters in.
fn pos(uri: &str, text: &str, needle: &str, nth: usize, delta: u32) -> Value {
    let (l, ch) = find(text, needle, nth);
    position_params(uri, l, ch + delta)
}

fn range_of(text: &str, needle: &str, nth: usize) -> lsp::Range {
    let (l, ch) = find(text, needle, nth);
    lsp::Range::new(
        lsp::Position::new(l, ch),
        lsp::Position::new(l, ch + needle.encode_utf16().count() as u32),
    )
}

/// The locations of a response, as (uri, range), sorted.
fn locations(v: &Value) -> Vec<(String, lsp::Range)> {
    let mut out: Vec<(String, lsp::Range)> = match v {
        Value::Array(xs) => xs
            .iter()
            .map(|x| {
                let l: lsp::Location = serde_json::from_value(x.clone()).unwrap();
                (l.uri.as_str().to_string(), l.range)
            })
            .collect(),
        Value::Null => Vec::new(),
        x => {
            let l: lsp::Location = serde_json::from_value(x.clone()).unwrap();
            vec![(l.uri.as_str().to_string(), l.range)]
        }
    };
    out.sort_by_key(|(u, r)| (u.clone(), r.start.line, r.start.character));
    out
}

fn references(
    c: &mut Client,
    uri: &str,
    text: &str,
    needle: &str,
    nth: usize,
    delta: u32,
    decl: bool,
) -> Vec<(String, lsp::Range)> {
    let mut p = pos(uri, text, needle, nth, delta);
    p["context"] = json!({ "includeDeclaration": decl });
    locations(&c.request::<lsp::request::References>(p))
}

/// The text after the edits of a workspace edit to one file.
fn apply(text: &str, edit: &Value, uri: &str) -> String {
    let edits: Vec<lsp::TextEdit> =
        serde_json::from_value(edit["changes"][uri].clone()).unwrap_or_default();
    apply_edits(text, edits)
}

fn apply_edits(text: &str, mut edits: Vec<lsp::TextEdit>) -> String {
    let ix = ilang_lsp::position::LineIndex::new(text);
    edits.sort_by_key(|e| std::cmp::Reverse((e.range.start.line, e.range.start.character)));
    let mut out = text.to_string();
    for e in edits {
        let s = ix.offset(e.range.start) as usize;
        let t = ix.offset(e.range.end) as usize;
        out.replace_range(s..t, &e.new_text);
    }
    out
}

fn hover_text(c: &mut Client, p: Value) -> String {
    let h = c.request::<lsp::request::HoverRequest>(p);
    h["contents"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn lsp_references_top_level_across_modules() {
    let ws = workspace(false);
    let mut c = ws.open();
    let refs = references(&mut c, &ws.main, &ws.main_text, "area(s)", 0, 1, true);
    let uris: Vec<&str> = refs.iter().map(|(u, _)| u.as_str()).collect();
    // The definition and its export in `shapes`, the import and the call
    // in `main`, and the use in the test module.
    assert!(
        uris.iter().filter(|u| **u == ws.shapes).count() >= 2,
        "{refs:?}"
    );
    assert_eq!(
        uris.iter().filter(|u| **u == ws.main).count(),
        2,
        "{refs:?}"
    );
    assert_eq!(
        uris.iter().filter(|u| **u == ws.test).count(),
        1,
        "{refs:?}"
    );
    assert!(
        refs.contains(&(ws.shapes.clone(), range_of(SHAPES, "area", 2))),
        "{refs:?}"
    );
    assert!(
        refs.contains(&(ws.main.clone(), range_of(&ws.main_text, "area", 1))),
        "{refs:?}"
    );
    c.shutdown();
}

#[test]
fn lsp_references_constructor() {
    let ws = workspace(false);
    let mut c = ws.open();
    let refs = references(&mut c, &ws.main, &ws.main_text, "Circle(radius", 0, 0, true);
    assert!(
        refs.contains(&(ws.shapes.clone(), range_of(SHAPES, "Circle", 0))),
        "{refs:?}"
    );
    // `Circle` in `Shape = Circle | Rect` names the labelled type `Circle`,
    // which is the constructor (syntax three §2.6).
    for nth in [1, 2] {
        assert!(
            refs.contains(&(ws.shapes.clone(), range_of(SHAPES, "Circle", nth))),
            "{refs:?}"
        );
    }
    assert!(
        refs.contains(&(ws.main.clone(), range_of(&ws.main_text, "Circle", 0))),
        "{refs:?}"
    );
    assert!(
        refs.contains(&(ws.main.clone(), range_of(&ws.main_text, "Circle", 1))),
        "{refs:?}"
    );
    assert!(
        refs.contains(&(ws.test.clone(), range_of(APP_TEST, "Circle", 0))),
        "{refs:?}"
    );
    c.shutdown();
}

#[test]
fn lsp_references_type() {
    let ws = workspace(false);
    let mut c = ws.open();
    let refs = references(&mut c, &ws.main, &ws.main_text, "Shape) ->", 0, 0, true);
    // The export, the definition, the annotation in `area`, and in `main`
    // the import and the annotation.
    for (uri, text, nth) in [
        (&ws.shapes, SHAPES, 0),
        (&ws.shapes, SHAPES, 2),
        (&ws.main, ws.main_text.as_str(), 0),
        (&ws.main, ws.main_text.as_str(), 1),
    ] {
        assert!(
            refs.contains(&(uri.clone(), range_of(text, "Shape", nth))),
            "{nth} {refs:?}"
        );
    }
    c.shutdown();
}

#[test]
fn lsp_references_field_label() {
    let ws = workspace(false);
    let mut c = ws.open();
    let refs = references(&mut c, &ws.main, &ws.main_text, "radius = 2", 0, 0, true);
    // The label of the field, the label of the call, the part of the
    // pattern in `area`, and the call in the test.
    assert!(
        refs.contains(&(ws.shapes.clone(), range_of(SHAPES, "radius", 0))),
        "{refs:?}"
    );
    assert!(
        refs.contains(&(ws.shapes.clone(), range_of(SHAPES, "radius", 1))),
        "{refs:?}"
    );
    assert!(
        refs.contains(&(ws.main.clone(), range_of(&ws.main_text, "radius", 0))),
        "{refs:?}"
    );
    assert!(
        refs.contains(&(ws.test.clone(), range_of(APP_TEST, "radius", 0))),
        "{refs:?}"
    );
    c.shutdown();
}

#[test]
fn lsp_references_type_parameter() {
    let ws = workspace(false);
    let mut c = Client::start();
    c.open(&ws.shapes, SHAPES);
    c.diagnostics(&ws.shapes);
    let refs = references(&mut c, &ws.shapes, SHAPES, "A) =", 0, 0, true);
    let (l, ch) = find(SHAPES, "item = A", 0);
    let use_ = lsp::Range::new(lsp::Position::new(l, ch + 7), lsp::Position::new(l, ch + 8));
    assert_eq!(
        refs,
        vec![
            (ws.shapes.clone(), range_of(SHAPES, "A) =", 0).with_len(1)),
            (ws.shapes.clone(), use_),
        ]
    );
    c.shutdown();
}

trait WithLen {
    fn with_len(self, n: u32) -> Self;
}

impl WithLen for lsp::Range {
    fn with_len(mut self, n: u32) -> Self {
        self.end = lsp::Position::new(self.start.line, self.start.character + n);
        self
    }
}

#[test]
fn lsp_references_module() {
    let ws = workspace(false);
    let mut c = ws.open();
    let refs = references(&mut c, &ws.main, &ws.main_text, "shapes.origin", 0, 2, true);
    assert!(
        refs.contains(&(ws.main.clone(), range_of(&ws.main_text, "shapes", 0))),
        "{refs:?}"
    );
    assert!(
        refs.contains(&(ws.main.clone(), range_of(&ws.main_text, "shapes", 2))),
        "{refs:?}"
    );
    c.shutdown();
}

#[test]
fn lsp_references_local_binding() {
    let ws = workspace(false);
    let mut c = ws.open();
    let refs = references(&mut c, &ws.main, &ws.main_text, "a + shapes", 0, 0, true);
    assert_eq!(
        refs,
        vec![
            (
                ws.main.clone(),
                range_of(&ws.main_text, "a = area", 0).with_len(1)
            ),
            (
                ws.main.clone(),
                range_of(&ws.main_text, "a + shapes", 0).with_len(1)
            ),
        ]
    );
    // Without the declaration.
    let refs = references(&mut c, &ws.main, &ws.main_text, "a + shapes", 0, 0, false);
    assert_eq!(refs.len(), 1, "{refs:?}");
    c.shutdown();
}

#[test]
fn lsp_references_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let refs = references(&mut c, &ws.main, &ws.main_text, "area s", 0, 1, true);
    assert!(
        refs.iter().filter(|(u, _)| *u == ws.main).count() >= 2,
        "{refs:?}\n{}",
        ws.main_text
    );
    assert!(
        refs.contains(&(ws.shapes.clone(), range_of(SHAPES, "area", 2))),
        "{refs:?}"
    );
    // A local name.
    let refs = references(&mut c, &ws.main, &ws.main_text, "(= a ", 0, 3, true);
    assert_eq!(refs.len(), 2, "{refs:?}");
    c.shutdown();
}

#[test]
fn lsp_references_into_dependency() {
    let ws = workspace(false);
    let mut c = ws.open();
    let refs = references(&mut c, &ws.main, &ws.main_text, "shout", 0, 1, true);
    assert!(
        refs.contains(&(ws.main.clone(), range_of(&ws.main_text, "shout", 0))),
        "{refs:?}"
    );
    assert!(refs.iter().any(|(u, _)| *u == ws.lib), "{refs:?}");
    c.shutdown();
}

// ---- Rename ----

fn rename(
    c: &mut Client,
    uri: &str,
    text: &str,
    needle: &str,
    nth: usize,
    delta: u32,
    new: &str,
) -> Result<Value, String> {
    let mut p = pos(uri, text, needle, nth, delta);
    p["newName"] = json!(new);
    c.try_request::<lsp::request::Rename>(p)
}

#[test]
fn lsp_rename_top_level_across_files() {
    let ws = workspace(false);
    let mut c = ws.open();
    let edit = rename(&mut c, &ws.main, &ws.main_text, "area(s)", 0, 0, "surface").unwrap();
    let main = apply(&ws.main_text, &edit, &ws.main);
    assert!(
        main.contains("(Shape, Circle, surface) = app/shapes"),
        "{main}"
    );
    assert!(main.contains("a = surface(s)"), "{main}");
    let shapes = apply(SHAPES, &edit, &ws.shapes);
    assert!(
        shapes.contains("\tShape(..)\n\tsurface\n\torigin"),
        "{shapes}"
    );
    assert!(shapes.contains("surface = (s = Shape)"), "{shapes}");
    let test = apply(APP_TEST, &edit, &ws.test);
    assert!(test.contains("shapes.surface(shapes.Circle"), "{test}");
    c.shutdown();
}

#[test]
fn lsp_rename_local_binding() {
    let ws = workspace(false);
    let mut c = ws.open();
    let edit = rename(&mut c, &ws.main, &ws.main_text, "a + shapes", 0, 0, "total").unwrap();
    let main = apply(&ws.main_text, &edit, &ws.main);
    assert!(
        main.contains("total = area(s)\n\ttotal + shapes.origin"),
        "{main}"
    );
    assert_eq!(edit["changes"].as_object().unwrap().len(), 1);
    c.shutdown();
}

#[test]
fn lsp_rename_constructor_and_type() {
    let ws = workspace(false);
    let mut c = ws.open();
    let edit = rename(
        &mut c,
        &ws.main,
        &ws.main_text,
        "Circle(radius",
        0,
        0,
        "Disk",
    )
    .unwrap();
    let main = apply(&ws.main_text, &edit, &ws.main);
    assert!(
        main.contains("(Shape, Disk, area)") && main.contains("c = Disk(radius = 2)"),
        "{main}"
    );
    let shapes = apply(SHAPES, &edit, &ws.shapes);
    assert!(
        shapes.contains("Disk = (radius = Int)") && shapes.contains("(Disk(radius = radius)) {"),
        "{shapes}"
    );
    assert!(apply(APP_TEST, &edit, &ws.test).contains("shapes.Disk(radius = 1)"));
    let edit = rename(&mut c, &ws.main, &ws.main_text, "Shape) ->", 0, 0, "Figure").unwrap();
    let shapes = apply(SHAPES, &edit, &ws.shapes);
    assert!(
        shapes.contains("\tFigure(..)\n\tarea")
            && shapes.contains("Figure = Circle | Rect")
            && shapes.contains("(s: Figure)"),
        "{shapes}"
    );
    let main = apply(&ws.main_text, &edit, &ws.main);
    assert!(
        main.contains("(Figure, Circle, area)") && main.contains("(s: Figure)"),
        "{main}"
    );
    c.shutdown();
}

#[test]
fn lsp_rename_field_keeps_pattern_names() {
    let ws = workspace(false);
    let mut c = ws.open();
    let edit = rename(&mut c, &ws.main, &ws.main_text, "radius = 2", 0, 0, "size").unwrap();
    let shapes = apply(SHAPES, &edit, &ws.shapes);
    assert!(shapes.contains("Circle = (size = Int)"), "{shapes}");
    // The pattern part keeps its local name and gets the new label.
    assert!(
        shapes.contains("(Circle(size = radius)) { radius * radius * 3 }"),
        "{shapes}"
    );
    assert!(apply(&ws.main_text, &edit, &ws.main).contains("Circle(size = 2)"));
    // Renaming that local name keeps the label.
    let mut c2 = Client::start();
    c2.open(&ws.shapes, SHAPES);
    c2.diagnostics(&ws.shapes);
    let edit = rename(&mut c2, &ws.shapes, SHAPES, "radius)) {", 0, 0, "r").unwrap();
    let shapes = apply(SHAPES, &edit, &ws.shapes);
    assert!(
        shapes.contains("(Circle(radius = r)) { r * r * 3 }"),
        "{shapes}"
    );
    c2.shutdown();
    c.shutdown();
}

#[test]
fn lsp_rename_module_name() {
    let ws = workspace(false);
    let mut c = ws.open();
    let edit = rename(
        &mut c,
        &ws.main,
        &ws.main_text,
        "shapes.origin",
        0,
        0,
        "geo",
    )
    .unwrap();
    let main = apply(&ws.main_text, &edit, &ws.main);
    assert!(
        main.contains("geo = (Shape, Circle, area) = app/shapes")
            && main.contains("a + geo.origin"),
        "{main}"
    );
    // A module imported without a name gets one.
    let edit = rename(&mut c, &ws.main, &ws.main_text, "text.shout", 0, 0, "words").unwrap();
    let main = apply(&ws.main_text, &edit, &ws.main);
    assert!(
        main.contains("words = lib/text") && main.contains("words.shout(\"hi\")"),
        "{main}"
    );
    c.shutdown();
}

#[test]
fn lsp_rename_refuses_dependencies_and_built_in_names() {
    let ws = workspace(false);
    let mut c = ws.open();
    let e = rename(&mut c, &ws.main, &ws.main_text, "shout", 0, 0, "yell").unwrap_err();
    assert!(e.contains("dependency `lib`"), "{e}");
    let mut p = pos(&ws.main, &ws.main_text, "shout", 0, 0);
    let e = c
        .try_request::<lsp::request::PrepareRenameRequest>(p.clone())
        .unwrap_err();
    assert!(e.contains("dependency"), "{e}");
    p = pos(&ws.main, &ws.main_text, "Int {", 0, 0);
    let e = c
        .try_request::<lsp::request::PrepareRenameRequest>(p)
        .unwrap_err();
    assert!(e.contains("built-in"), "{e}");
    c.shutdown();
}

#[test]
fn lsp_rename_refuses_the_library() {
    let dir = tempfile::tempdir().unwrap();
    let text = "main = () { panic(\"no\") }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let e = rename(&mut c, &main, text, "panic", 0, 1, "stop").unwrap_err();
    assert!(e.contains("library `i`"), "{e}");
    c.shutdown();
}

#[test]
fn lsp_rename_refuses_macro_expansion() {
    let dir = tempfile::tempdir().unwrap();
    let main_text = "(m = app/macros)\nmain = () {\n  x = 42\n  m.pass(x)\n}\n";
    let macro_text = "(, (= syntax (/ i syntax)))\n(, pass)\n(@ macro (= pass (-> (, (: x (. syntax Syntax))) (. syntax Syntax) x)))\n";
    package(
        dir.path(),
        &[("src/main.i", main_text), ("src/macros.is", macro_text)],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, main_text);
    c.diagnostics(&main);
    // The macro, whose calls are expanded, is refused.
    let e = rename(&mut c, &main, main_text, "pass(x)", 0, 1, "keep").unwrap_err();
    assert!(e.contains("macro"), "{e}");
    let e = c
        .try_request::<lsp::request::PrepareRenameRequest>(pos(&main, main_text, "pass(x)", 0, 1))
        .unwrap_err();
    assert!(e.contains("macro"), "{e}");
    // The argument of the macro call is the user's name: it may be renamed.
    let edit = rename(&mut c, &main, main_text, "x = 42", 0, 0, "y").unwrap();
    let after = apply(main_text, &edit, &main);
    assert!(
        after.contains("y = 42") && after.contains("m.pass(y)"),
        "{after}"
    );
    c.shutdown();
}

#[test]
fn lsp_rename_rejects_names_of_the_wrong_kind() {
    let ws = workspace(false);
    let mut c = ws.open();
    let e = rename(&mut c, &ws.main, &ws.main_text, "area(s)", 0, 0, "Area").unwrap_err();
    assert!(e.contains("not a valid new name"), "{e}");
    let e = rename(
        &mut c,
        &ws.main,
        &ws.main_text,
        "Circle(radius",
        0,
        0,
        "disk",
    )
    .unwrap_err();
    assert!(e.contains("UpName"), "{e}");
    let e = rename(&mut c, &ws.main, &ws.main_text, "a + shapes", 0, 0, "1x").unwrap_err();
    assert!(e.contains("not a valid new name"), "{e}");
    c.shutdown();
}

#[test]
fn lsp_rename_rejects_capture_and_clashes() {
    let ws = workspace(false);
    let mut c = ws.open();
    // `s` is the parameter in scope where `a` is used.
    let e = rename(&mut c, &ws.main, &ws.main_text, "a + shapes", 0, 0, "s").unwrap_err();
    assert!(e.contains("cannot rename to `s`"), "{e}");
    // `size` is another top-level name of `main`.
    let e = rename(&mut c, &ws.main, &ws.main_text, "main = ", 0, 0, "size").unwrap_err();
    assert!(e.contains("already has a top-level name"), "{e}");
    // `n` in `main` would cover `c` ... the uses of `size` in the scope
    // of a local named `size` would be captured.
    let e = rename(&mut c, &ws.main, &ws.main_text, "c = Circle", 0, 0, "size").unwrap_err();
    assert!(e.contains("captured") || e.contains("cannot rename"), "{e}");
    c.shutdown();
}

#[test]
fn lsp_rename_prepare_gives_the_name() {
    let ws = workspace(false);
    let mut c = ws.open();
    let r = c.request::<lsp::request::PrepareRenameRequest>(pos(
        &ws.main,
        &ws.main_text,
        "size(c)",
        0,
        2,
    ));
    assert_eq!(r["placeholder"], "size");
    let range: lsp::Range = serde_json::from_value(r["range"].clone()).unwrap();
    assert_eq!(range, range_of(&ws.main_text, "size", 1));
    // Nothing to rename on a literal.
    let r = c.request::<lsp::request::PrepareRenameRequest>(pos(
        &ws.main,
        &ws.main_text,
        "\"hi\"",
        0,
        1,
    ));
    assert_eq!(r, Value::Null);
    c.shutdown();
}

#[test]
fn lsp_rename_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let edit = rename(&mut c, &ws.main, &ws.main_text, "area s", 0, 0, "surface").unwrap();
    let main = apply(&ws.main_text, &edit, &ws.main);
    assert!(
        main.contains("surface s") && !main.contains("area"),
        "{main}"
    );
    assert!(apply(SHAPES, &edit, &ws.shapes).contains("surface = (s = Shape)"));
    // A local.
    let edit = rename(&mut c, &ws.main, &ws.main_text, "(= a ", 0, 3, "total").unwrap();
    let main = apply(&ws.main_text, &edit, &ws.main);
    assert!(
        main.contains("(= total ") && main.contains("total (. shapes origin)")
            || main.matches("total").count() == 2,
        "{main}"
    );
    c.shutdown();
}

// ---- Type definition, implementation, definition ----

#[test]
fn lsp_definition_type_of_a_local() {
    let ws = workspace(false);
    let mut c = ws.open();
    let r = c.request::<lsp::request::GotoTypeDefinition>(pos(&ws.main, &ws.main_text, "c)", 0, 0));
    assert_eq!(
        locations(&r),
        vec![(ws.shapes.clone(), range_of(SHAPES, "Shape", 1))]
    );
    // Of a constructor: its type.
    let r = c.request::<lsp::request::GotoTypeDefinition>(pos(
        &ws.main,
        &ws.main_text,
        "Circle(radius",
        0,
        1,
    ));
    assert_eq!(
        locations(&r),
        vec![(ws.shapes.clone(), range_of(SHAPES, "Shape", 1))]
    );
    c.shutdown();
}

#[test]
fn lsp_definition_implementation_of_a_type() {
    let ws = workspace(false);
    let mut c = ws.open();
    let r = c.request::<lsp::request::GotoImplementation>(pos(
        &ws.main,
        &ws.main_text,
        "Shape) ->",
        0,
        1,
    ));
    assert_eq!(
        locations(&r),
        vec![
            (ws.shapes.clone(), range_of(SHAPES, "Circle", 0)),
            (ws.shapes.clone(), range_of(SHAPES, "Rect", 0)),
        ]
    );
    c.shutdown();
}

#[test]
fn lsp_definition_implementation_of_a_service() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Clock = (now = () -> Int)\n\nelapsed = (start: Int) <- (clock: Clock) { clock.now() - start }\n\nfixed = () { elapsed(40) <- (Clock(now = () { 42 })) }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let r = c.request::<lsp::request::GotoImplementation>(pos(&main, text, "Clock)", 0, 0));
    let found = locations(&r);
    assert!(
        found
            .iter()
            .any(|(u, r)| *u == main && r.start == range_of(text, "Clock(now", 0).start),
        "{found:?}"
    );
    let (l, _) = find(text, "<- (Clock(now", 0);
    assert!(found.iter().any(|(_, r)| r.start.line == l), "{found:?}");
    c.shutdown();
}

#[test]
fn lsp_definition_through_imports() {
    let ws = workspace(false);
    let mut c = ws.open();
    // A name put in by an import.
    let d = c.request::<lsp::request::GotoDefinition>(pos(&ws.main, &ws.main_text, "area)", 0, 1));
    assert_eq!(
        locations(&d),
        vec![(ws.shapes.clone(), range_of(SHAPES, "area", 2))]
    );
    // The path of an import: the module.
    let d =
        c.request::<lsp::request::GotoDefinition>(pos(&ws.main, &ws.main_text, "app/shapes", 0, 5));
    assert_eq!(locations(&d)[0].0, ws.shapes);
    // A module name before a member.
    let d = c.request::<lsp::request::GotoDefinition>(pos(
        &ws.main,
        &ws.main_text,
        "shapes.origin",
        0,
        1,
    ));
    assert_eq!(locations(&d)[0].0, ws.shapes);
    c.shutdown();
}

#[test]
fn lsp_definition_into_a_dependency() {
    let ws = workspace(false);
    let mut c = ws.open();
    let d = c.request::<lsp::request::GotoDefinition>(pos(&ws.main, &ws.main_text, "shout", 0, 2));
    assert_eq!(
        locations(&d),
        vec![(ws.lib.clone(), range_of(LIB_TEXT, "shout", 1))]
    );
    c.shutdown();
}

const MACRO_MAIN: &str = "(m = app/macros)\nmain = () {\n  x = 42\n  m.pass(x)\n}\n";
const MACROS: &str = "(, (= syntax (/ i syntax)))\n(, pass)\n/// Gives back its argument.\n(@ macro (= pass (-> (, (: x (. syntax Syntax))) (. syntax Syntax) x)))\n";

#[test]
fn lsp_definition_of_a_macro_call() {
    let dir = tempfile::tempdir().unwrap();
    package(
        dir.path(),
        &[("src/main.i", MACRO_MAIN), ("src/macros.is", MACROS)],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let macros = file_uri(&dir.path().join("src/macros.is"));
    let mut c = Client::start();
    c.open(&main, MACRO_MAIN);
    c.diagnostics(&main);
    let d = c.request::<lsp::request::GotoDefinition>(pos(&main, MACRO_MAIN, "pass(x)", 0, 1));
    assert_eq!(
        locations(&d),
        vec![(macros.clone(), range_of(MACROS, "pass", 1))]
    );
    c.shutdown();
}

#[test]
fn lsp_definition_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let d = c.request::<lsp::request::GotoDefinition>(pos(&ws.main, &ws.main_text, "area s", 0, 1));
    assert_eq!(
        locations(&d),
        vec![(ws.shapes.clone(), range_of(SHAPES, "area", 2))]
    );
    let d = c.request::<lsp::request::GotoDefinition>(pos(&ws.main, &ws.main_text, "shout", 0, 1));
    assert_eq!(
        locations(&d),
        vec![(ws.lib.clone(), range_of(LIB_TEXT, "shout", 1))]
    );
    let r =
        c.request::<lsp::request::GotoTypeDefinition>(pos(&ws.main, &ws.main_text, "size c", 0, 5));
    assert_eq!(
        locations(&r),
        vec![(ws.shapes.clone(), range_of(SHAPES, "Shape", 1))]
    );
    c.shutdown();
}

// ---- Completion ----

fn completion(
    c: &mut Client,
    uri: &str,
    text: &str,
    needle: &str,
    nth: usize,
    delta: u32,
) -> Vec<lsp::CompletionItem> {
    let r = c.request::<lsp::request::Completion>(pos(uri, text, needle, nth, delta));
    match serde_json::from_value::<lsp::CompletionResponse>(r).unwrap() {
        lsp::CompletionResponse::Array(xs) => xs,
        lsp::CompletionResponse::List(l) => l.items,
    }
}

fn item<'a>(items: &'a [lsp::CompletionItem], label: &str) -> &'a lsp::CompletionItem {
    items.iter().find(|i| i.label == label).unwrap_or_else(|| {
        panic!(
            "no {label} in {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        )
    })
}

#[test]
fn lsp_completion_module_members() {
    let ws = workspace(false);
    let mut c = ws.open();
    let typing = ws.main_text.replace("shapes.origin", "shapes.");
    c.change(&ws.main, 2, &typing);
    let items = completion(&mut c, &ws.main, &typing, "shapes.\n", 0, 7);
    let area = item(&items, "area");
    assert_eq!(area.detail.as_deref(), Some("(s = Shape) -> Int"));
    item(&items, "origin");
    item(&items, "Circle");
    c.shutdown();
}

#[test]
fn lsp_completion_fields_of_a_value() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Point = (x = Int, y = Int)\n\nf = (p: Point) { p.x }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let items = completion(&mut c, &main, text, "p.x", 0, 3);
    assert_eq!(
        items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
        vec!["x", "y"]
    );
    assert_eq!(items[0].detail.as_deref(), Some("Int"));
    c.shutdown();
}

#[test]
fn lsp_completion_constructors_in_a_match() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Circle = (Int)\nRect = (width = Int, height = Int)\nDot = ()\nShape = Circle | Rect | Dot\n\nf = (s: Shape) { s |> (Dot) { 0 } | (Circle(1)) { 1 } | () { 2 } }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let items = completion(&mut c, &main, text, "() { 2", 0, 1);
    let circle = item(&items, "Circle");
    assert_eq!(circle.insert_text.as_deref(), Some("Circle(${1:_})"));
    assert_eq!(
        circle.insert_text_format,
        Some(lsp::InsertTextFormat::SNIPPET)
    );
    assert_eq!(
        item(&items, "Rect").insert_text.as_deref(),
        Some("Rect(..)")
    );
    assert_eq!(item(&items, "Dot").insert_text.as_deref(), Some("Dot"));
    item(&items, "_");
    // The constructors come first.
    assert_eq!(items[0].label, "Circle");
    c.shutdown();
}

#[test]
fn lsp_completion_labels_in_a_call() {
    let ws = workspace(false);
    let mut c = ws.open();
    let typing = ws.main_text.replace("Circle(radius = 2)", "Circle()");
    c.change(&ws.main, 2, &typing);
    let items = completion(&mut c, &ws.main, &typing, "Circle()", 0, 7);
    let radius = item(&items, "radius =");
    assert_eq!(radius.insert_text.as_deref(), Some("radius = "));
    assert_eq!(radius.detail.as_deref(), Some("Int"));
    assert_eq!(items[0].label, "radius =");
    // Locals are there too.
    item(&items, "size");
    c.shutdown();
}

#[test]
fn lsp_completion_import_paths() {
    let ws = workspace(false);
    let mut c = ws.open();
    let typing = ws.main_text.replace("\tlib/text\n", "\tlib/te\n");
    c.change(&ws.main, 2, &typing);
    let items = completion(&mut c, &ws.main, &typing, "lib/te", 0, 6);
    let text = item(&items, "lib/text");
    let doc = match &text.documentation {
        Some(lsp::Documentation::MarkupContent(m)) => m.value.clone(),
        other => panic!("{other:?}"),
    };
    assert!(doc.contains("shout") && doc.contains("Tone"), "{doc}");
    match &text.text_edit {
        Some(lsp::CompletionTextEdit::Edit(e)) => {
            assert_eq!(e.range, range_of(&typing, "lib/te", 0));
            assert_eq!(e.new_text, "lib/text");
        }
        other => panic!("{other:?}"),
    }
    item(&items, "app/shapes");
    item(&items, "i/task");
    // A test module is not offered to a module of `src`.
    assert!(!items.iter().any(|i| i.label.contains("shapes_test")));
    c.shutdown();
}

#[test]
fn lsp_completion_import_names() {
    let ws = workspace(false);
    let mut c = ws.open();
    let items = completion(&mut c, &ws.main, &ws.main_text, "Circle, area)", 0, 0);
    item(&items, "origin");
    item(&items, "Rect");
    item(&items, "Box");
    c.shutdown();
}

#[test]
fn lsp_completion_services_in_an_environment() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Clock = (now = () -> Int)\nPoint = { (x = Int) }\n\nelapsed = (start: Int) <- (clock: Clock) { clock.now() - start }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let items = completion(&mut c, &main, text, "Clock) {", 0, 0);
    assert_eq!(
        items[0].label,
        "Clock",
        "{:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    assert!(!items.iter().any(|i| i.label == "Point"));
    assert!(!items.iter().any(|i| i.label == "elapsed"));
    c.shutdown();
}

#[test]
fn lsp_completion_types_in_a_type_position() {
    let ws = workspace(false);
    let mut c = ws.open();
    let items = completion(&mut c, &ws.main, &ws.main_text, "Shape) ->", 0, 2);
    item(&items, "Shape");
    item(&items, "Int");
    item(&items, "List");
    // Values are not types.
    assert!(!items.iter().any(|i| i.label == "size" || i.label == "main"));
    c.shutdown();
}

#[test]
fn lsp_completion_resolve_gives_documentation() {
    let ws = workspace(false);
    let mut c = ws.open();
    let items = completion(&mut c, &ws.main, &ws.main_text, "a + shapes", 0, 0);
    let size = item(&items, "size").clone();
    assert_eq!(size.detail.as_deref(), Some("(s = Shape) -> Int"));
    let resolved =
        c.request::<lsp::request::ResolveCompletionItem>(serde_json::to_value(&size).unwrap());
    assert_eq!(resolved["documentation"]["value"], "How big a shape is.");
    c.shutdown();
}

#[test]
fn lsp_completion_imports_a_name() {
    let ws = workspace(false);
    let mut c = ws.open();
    let typing = ws.main_text.replace("\tn\n}", "\tn + ori\n}");
    c.change(&ws.main, 2, &typing);
    let items = completion(&mut c, &ws.main, &typing, "ori\n}", 0, 3);
    // `origin` is in `app/shapes`, which is imported: written with the
    // module. `Loud` of `lib/text` too.
    assert!(
        !items
            .iter()
            .any(|i| i.label == "origin" && i.additional_text_edits.is_some())
    );
    let typing = ws
        .main_text
        .replace("\n\tlib/text\n", "\n")
        .replace("\tn\n}", "\tn + sho\n}")
        .replace("\tt = text.shout(\"hi\")\n", "");
    c.change(&ws.main, 3, &typing);
    let items = completion(&mut c, &ws.main, &typing, "sho\n}", 0, 3);
    let shout = item(&items, "shout");
    let edits = shout.additional_text_edits.clone().unwrap();
    let after = apply_edits(&typing, edits);
    assert!(after.contains("(shout) = lib/text"), "{after}");
    c.shutdown();
}

#[test]
fn lsp_completion_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let items = completion(&mut c, &ws.main, &ws.main_text, "a (. shapes", 0, 1);
    item(&items, "a");
    item(&items, "s");
    item(&items, "size");
    let size = item(&items, "size");
    assert_eq!(size.detail.as_deref(), Some("(s = Shape) -> Int"));
    c.shutdown();
}

// ---- Signature help ----

fn signature(c: &mut Client, p: Value) -> lsp::SignatureHelp {
    let r = c.request::<lsp::request::SignatureHelpRequest>(p);
    serde_json::from_value(r).unwrap()
}

#[test]
fn lsp_signature_of_a_function() {
    let dir = tempfile::tempdir().unwrap();
    let text = "/// Adds.\nadd = (x: Int, y: Int) -> Int { x + y }\n\nmain = () { add(1, 2) }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let s = signature(&mut c, pos(&main, text, "add(1", 0, 4));
    assert_eq!(s.signatures[0].label, "add(x = Int, y = Int) -> Int");
    assert_eq!(s.active_parameter, Some(0));
    let params = s.signatures[0].parameters.clone().unwrap();
    assert_eq!(params[1].label, lsp::ParameterLabel::LabelOffsets([13, 20]));
    let s = signature(&mut c, pos(&main, text, "2)", 0, 0));
    assert_eq!(s.active_parameter, Some(1));
    assert!(
        matches!(&s.signatures[0].documentation, Some(lsp::Documentation::MarkupContent(m)) if m.value == "Adds.")
    );
    c.shutdown();
}

#[test]
fn lsp_signature_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let text = "make = (name: String, age: Int) { (name, age) }\n\nmain = () { make(\"a\", 3) }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let s = signature(&mut c, pos(&main, text, "3)", 0, 0));
    assert_eq!(s.active_parameter, Some(1));
    let s = signature(&mut c, pos(&main, text, "\"a\"", 0, 1));
    assert_eq!(s.active_parameter, Some(0));
    c.shutdown();
}

#[test]
fn lsp_signature_of_a_constructor() {
    let ws = workspace(false);
    let mut c = ws.open();
    let s = signature(&mut c, pos(&ws.main, &ws.main_text, "radius = 2", 0, 9));
    assert_eq!(s.signatures[0].label, "Circle(radius = Int) -> Shape");
    assert_eq!(s.active_parameter, Some(0));
    c.shutdown();
}

#[test]
fn lsp_signature_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let s = signature(&mut c, pos(&ws.main, &ws.main_text, "area s", 0, 5));
    assert_eq!(s.signatures[0].label, "area(s = Shape) -> Int");
    assert_eq!(s.active_parameter, Some(0));
    c.shutdown();
}

// ---- Hover ----

#[test]
fn lsp_hover_type_and_documentation() {
    let ws = workspace(false);
    let mut c = ws.open();
    let h = hover_text(&mut c, pos(&ws.main, &ws.main_text, "area(s)", 0, 1));
    assert!(
        h.contains("area: (s = Shape) -> Int") && h.contains("The area of a shape."),
        "{h}"
    );
    let h = hover_text(&mut c, pos(&ws.main, &ws.main_text, "Circle(radius", 0, 1));
    assert!(
        h.contains("Circle: (radius: Int) -> Shape") && h.contains("A shape in the plane."),
        "{h}"
    );
    c.shutdown();
}

#[test]
fn lsp_hover_value_of_a_constant() {
    let ws = workspace(false);
    let mut c = ws.open();
    let h = hover_text(&mut c, pos(&ws.main, &ws.main_text, "origin", 0, 1));
    assert!(h.contains("origin: Int"), "{h}");
    assert!(h.contains("Value: `0`"), "{h}");
    assert!(h.contains("The place where shapes start."), "{h}");
    c.shutdown();
}

#[test]
fn lsp_hover_errors_and_environment() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Clock = (now = () -> Int)\n\nelapsed = (start: Int) <- (clock: Clock) { clock.now() - start }\n\nparse = (s: String) <- (clock: Clock, fail: Fail(String)) -> Int { 0 }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let h = hover_text(&mut c, pos(&main, text, "elapsed", 0, 1));
    assert!(h.contains("Services: `Clock`"), "{h}");
    assert!(!h.contains("Possible errors:"), "{h}");
    let h = hover_text(&mut c, pos(&main, text, "parse", 0, 1));
    assert!(
        h.contains("Possible errors: `String`") && h.contains("Services: `Clock`"),
        "{h}"
    );
    c.shutdown();
}

#[test]
fn lsp_hover_declared_type_of_a_union() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Answer = \"yes\" | \"no\"\n\nmain = () {\n\tx: Answer = \"yes\"\n\tx\n}\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let h = hover_text(&mut c, pos(&main, text, "x\n}", 0, 0));
    assert!(h.contains("Answer"), "{h}");
    let h = hover_text(&mut c, pos(&main, text, "Answer =", 0, 1));
    assert!(h.contains("\"yes\" | \"no\""), "{h}");
    c.shutdown();
}

#[test]
fn lsp_hover_macro_expansion() {
    let dir = tempfile::tempdir().unwrap();
    package(
        dir.path(),
        &[("src/main.i", MACRO_MAIN), ("src/macros.is", MACROS)],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, MACRO_MAIN);
    c.diagnostics(&main);
    let h = hover_text(&mut c, pos(&main, MACRO_MAIN, "pass(x)", 0, 1));
    assert!(h.contains("macro pass"), "{h}");
    assert!(h.contains("Macro from `app/macros`"), "{h}");
    assert!(h.contains("Expands to:"), "{h}");
    c.shutdown();
}

#[test]
fn lsp_hover_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let h = hover_text(&mut c, pos(&ws.main, &ws.main_text, "area s", 0, 1));
    assert!(
        h.contains("area: (s = Shape) -> Int") && h.contains("The area of a shape."),
        "{h}"
    );
    let h = hover_text(&mut c, pos(&ws.main, &ws.main_text, "(= a ", 0, 3));
    assert!(h.contains("a: Int"), "{h}");
    c.shutdown();
}

// ---- Code actions ----

fn code_actions(c: &mut Client, uri: &str, range: lsp::Range) -> Vec<lsp::CodeAction> {
    let r = c.request::<lsp::request::CodeActionRequest>(json!({
        "textDocument": { "uri": uri },
        "range": range,
        "context": { "diagnostics": [] }
    }));
    serde_json::from_value::<Vec<lsp::CodeActionOrCommand>>(r)
        .unwrap()
        .into_iter()
        .map(|a| match a {
            lsp::CodeActionOrCommand::CodeAction(a) => a,
            lsp::CodeActionOrCommand::Command(c) => panic!("{c:?}"),
        })
        .collect()
}

fn action<'a>(actions: &'a [lsp::CodeAction], title: &str) -> &'a lsp::CodeAction {
    actions
        .iter()
        .find(|a| a.title.contains(title))
        .unwrap_or_else(|| {
            panic!(
                "no {title} in {:?}",
                actions.iter().map(|a| &a.title).collect::<Vec<_>>()
            )
        })
}

fn applied(a: &lsp::CodeAction, text: &str, uri: &str) -> String {
    apply(
        text,
        &serde_json::to_value(a.edit.as_ref().unwrap()).unwrap(),
        uri,
    )
}

#[test]
fn lsp_code_action_adds_a_missing_import() {
    let ws = workspace(false);
    let mut c = ws.open();
    let broken = ws.main_text.replace("\tn\n}", "\tn + origin\n}");
    c.change(&ws.main, 2, &broken);
    let diags = c.diagnostics(&ws.main);
    let d = diags
        .iter()
        .find(|d| d.code == Some(lsp::NumberOrString::String("E0201".into())))
        .unwrap();
    assert_eq!(d.data, Some(json!({ "fix": "import" })));
    let actions = code_actions(&mut c, &ws.main, d.range);
    let a = action(&actions, "Import `origin` from `app/shapes`");
    assert_eq!(a.diagnostics.as_ref().unwrap()[0].code, d.code);
    let after = applied(a, &broken, &ws.main);
    assert!(
        after.contains("shapes = (Shape, Circle, area, origin) = app/shapes"),
        "{after}"
    );
    c.change(&ws.main, 3, &after);
    let diags = c.diagnostics(&ws.main);
    assert!(
        !diags
            .iter()
            .any(|d| d.severity == Some(lsp::DiagnosticSeverity::ERROR)),
        "{diags:?}"
    );
    c.shutdown();
}

#[test]
fn lsp_code_action_imports_a_module() {
    let dir = tempfile::tempdir().unwrap();
    let util = "(double)\n\ndouble = (n: Int) -> Int { n * 2 }\n";
    let text = "main = () { util.double(2) }\n";
    package(dir.path(), &[("src/main.i", text), ("src/util.i", util)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    let diags = c.diagnostics(&main);
    let d = diags
        .iter()
        .find(|d| d.code == Some(lsp::NumberOrString::String("E0201".into())))
        .unwrap();
    let actions = code_actions(&mut c, &main, d.range);
    let after = applied(action(&actions, "Import `app/util`"), text, &main);
    assert_eq!(after, "(app/util)\n\nmain = () { util.double(2) }\n");
    c.shutdown();
}

#[test]
fn lsp_code_action_removes_an_unused_binding() {
    let dir = tempfile::tempdir().unwrap();
    let text = "/// Not used.\nhelper = (n: Int) { n }\n\nmain = () { 1 }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    let diags = c.diagnostics(&main);
    let d = diags
        .iter()
        .find(|d| d.message.contains("`helper`"))
        .unwrap();
    assert_eq!(d.tags, Some(vec![lsp::DiagnosticTag::UNNECESSARY]));
    let actions = code_actions(&mut c, &main, d.range);
    let after = applied(
        action(&actions, "Remove the unused binding `helper`"),
        text,
        &main,
    );
    assert_eq!(after, "main = () { 1 }\n");
    c.shutdown();
}

#[test]
fn lsp_code_action_removes_an_unused_import() {
    let ws = workspace(false);
    let mut c = ws.open();
    let text = ws.main_text.replace("\tt = text.shout(\"hi\")\n", "");
    c.change(&ws.main, 2, &text);
    c.diagnostics(&ws.main);
    let actions = code_actions(&mut c, &ws.main, range_of(&text, "lib/text", 0));
    let after = applied(
        action(&actions, "Remove the unused import `lib/text`"),
        &text,
        &ws.main,
    );
    assert!(
        after.starts_with("(\n\tshapes = (Shape, Circle, area) = app/shapes\n)"),
        "{after}"
    );
    // A used import has no such action.
    let actions = code_actions(&mut c, &ws.main, range_of(&text, "app/shapes", 0));
    assert!(
        !actions
            .iter()
            .any(|a| a.title.contains("Remove the unused import"))
    );
    c.shutdown();
}

#[test]
fn lsp_code_action_adds_the_missing_clauses() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Circle = (Int)\nRect = (width = Int, height = Int)\nDot = ()\nShape = Circle | Rect | Dot\n\nf = (s: Shape) { s |> (Dot) { 0 } | (Circle(1)) { 1 } }\n\nmain = () { f(Dot) }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    let diags = c.diagnostics(&main);
    let d = diags
        .iter()
        .find(|d| d.code == Some(lsp::NumberOrString::String("E0516".into())))
        .unwrap();
    assert_eq!(d.data, Some(json!({ "fix": "clauses" })));
    let actions = code_actions(&mut c, &main, d.range);
    let after = applied(action(&actions, "Add the missing clauses"), text, &main);
    assert!(
        after.contains("| (Circle(1)) { 1 } | (Circle(..)) { panic(\"todo\") } | (Rect(..)) { panic(\"todo\") }"),
        "{after}"
    );
    // The new text checks.
    c.change(&main, 2, &after);
    let diags = c.diagnostics(&main);
    assert!(
        !diags
            .iter()
            .any(|d| d.severity == Some(lsp::DiagnosticSeverity::ERROR)),
        "{diags:?}"
    );
    c.shutdown();
}

#[test]
fn lsp_code_action_adds_a_type_annotation() {
    let ws = workspace(false);
    let mut c = ws.open();
    let actions = code_actions(
        &mut c,
        &ws.main,
        range_of(&ws.main_text, "a = area", 0).with_len(1),
    );
    let a = action(&actions, "Add the type annotation `Int`");
    assert_eq!(a.kind, Some(lsp::CodeActionKind::REFACTOR_REWRITE));
    let after = applied(a, &ws.main_text, &ws.main);
    assert!(after.contains("\ta: Int = area(s)"), "{after}");
    // Annotated already: no action.
    let actions = code_actions(
        &mut c,
        &ws.main,
        range_of(&ws.main_text, "s: Shape", 0).with_len(1),
    );
    assert!(!actions.iter().any(|a| a.title.contains("type annotation")));
    c.shutdown();
}

#[test]
fn lsp_code_action_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let broken = to_lisp(&APP_MAIN.replace("\tn\n}", "\tn + origin\n}"));
    c.change(&ws.main, 2, &broken);
    let diags = c.diagnostics(&ws.main);
    let d = diags
        .iter()
        .find(|d| d.code == Some(lsp::NumberOrString::String("E0201".into())))
        .unwrap();
    let actions = code_actions(&mut c, &ws.main, d.range);
    let after = applied(action(&actions, "Import `origin`"), &broken, &ws.main);
    assert!(after.contains("(, Shape Circle area origin)"), "{after}");
    c.change(&ws.main, 3, &after);
    let diags = c.diagnostics(&ws.main);
    assert!(
        !diags
            .iter()
            .any(|d| d.severity == Some(lsp::DiagnosticSeverity::ERROR)),
        "{diags:?}\n{after}"
    );
    // A type annotation, in the syntax of the surface.
    let (l, ch) = find(&after, "(= a ", 0);
    let at = lsp::Position::new(l, ch + 3);
    let actions = code_actions(&mut c, &ws.main, lsp::Range::new(at, at));
    let annotated = applied(
        action(&actions, "Add the type annotation `Int`"),
        &after,
        &ws.main,
    );
    assert!(annotated.contains("(= (: a Int) "), "{annotated}");
    c.change(&ws.main, 4, &annotated);
    let diags = c.diagnostics(&ws.main);
    assert!(
        !diags
            .iter()
            .any(|d| d.severity == Some(lsp::DiagnosticSeverity::ERROR)),
        "{diags:?}\n{annotated}"
    );
    c.shutdown();
}

// ---- Inlay hints, folding, selection, highlights, symbols, links ----

fn inlay(c: &mut Client, uri: &str, text: &str) -> Vec<lsp::InlayHint> {
    let ix = ilang_lsp::position::LineIndex::new(text);
    let r = c.request::<lsp::request::InlayHintRequest>(json!({
        "textDocument": { "uri": uri },
        "range": { "start": { "line": 0, "character": 0 }, "end": ix.position(text.len() as u32) }
    }));
    serde_json::from_value(r).unwrap()
}

fn label(h: &lsp::InlayHint) -> String {
    match &h.label {
        lsp::InlayHintLabel::String(s) => s.clone(),
        lsp::InlayHintLabel::LabelParts(p) => p.iter().map(|x| x.value.clone()).collect(),
    }
}

#[test]
fn lsp_inlay_types_of_bindings_and_parameters() {
    let dir = tempfile::tempdir().unwrap();
    let text = "add = (x: Int, y) { x + y }\n\nmain = () {\n\tn = add(1, 2)\n\tk = 3\n\tn + k\n}\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    c.diagnostics(&main);
    let hints = inlay(&mut c, &main, text);
    let n = hints
        .iter()
        .find(|h| h.position == range_of(text, "\tn =", 0).with_len(2).end)
        .unwrap();
    assert_eq!(label(n), ": Int");
    assert_eq!(n.kind, Some(lsp::InlayHintKind::TYPE));
    let y = hints
        .iter()
        .find(|h| h.position == range_of(text, "y)", 0).with_len(1).end)
        .unwrap();
    assert_eq!(label(y), ": Int");
    assert_eq!(y.kind, Some(lsp::InlayHintKind::PARAMETER));
    // Not for a literal, and not for an annotated name.
    assert!(
        !hints
            .iter()
            .any(|h| h.position == range_of(text, "k =", 0).with_len(1).end)
    );
    assert!(
        !hints
            .iter()
            .any(|h| h.position == range_of(text, "x:", 0).with_len(1).end)
    );
    c.shutdown();
}

#[test]
fn lsp_inlay_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let hints = inlay(&mut c, &ws.main, &ws.main_text);
    assert!(hints.iter().any(|h| label(h) == ": Int"), "{hints:?}");
    c.shutdown();
}

#[test]
fn lsp_folding_ranges() {
    let ws = workspace(false);
    let mut c = Client::start();
    c.open(&ws.shapes, SHAPES);
    c.diagnostics(&ws.shapes);
    let r = c.request::<lsp::request::FoldingRangeRequest>(
        json!({ "textDocument": { "uri": ws.shapes } }),
    );
    let ranges: Vec<lsp::FoldingRange> = serde_json::from_value(r).unwrap();
    let (area, _) = find(SHAPES, "area = ", 0);
    let (case, _) = find(SHAPES, "s |>", 0);
    assert!(ranges.iter().any(|f| f.start_line == area), "{ranges:?}");
    // A match written over several lines as a chain `s |> (p) { … } | …`
    // folds from its subject to its last branch.
    assert!(
        ranges
            .iter()
            .any(|f| f.start_line == case && f.end_line == case + 2),
        "{ranges:?}"
    );
    // Imports.
    let mut c2 = ws.open();
    let r = c2.request::<lsp::request::FoldingRangeRequest>(
        json!({ "textDocument": { "uri": ws.main } }),
    );
    let ranges: Vec<lsp::FoldingRange> = serde_json::from_value(r).unwrap();
    assert!(
        // The header group `( … )` of the imports folds whole.
        ranges.iter().any(|f| f.start_line == 0
            && f.end_line == 3
            && f.kind == Some(lsp::FoldingRangeKind::Imports)),
        "{ranges:?}"
    );
    c2.shutdown();
    c.shutdown();
}

#[test]
fn lsp_folding_selection_ranges() {
    let ws = workspace(false);
    let mut c = ws.open();
    let (l, ch) = find(&ws.main_text, "area(s)", 0);
    let r = c.request::<lsp::request::SelectionRangeRequest>(json!({
        "textDocument": { "uri": ws.main },
        "positions": [{ "line": l, "character": ch + 5 }]
    }));
    let ranges: Vec<lsp::SelectionRange> = serde_json::from_value(r).unwrap();
    let mut chain = Vec::new();
    let mut cur = Some(&ranges[0]);
    while let Some(r) = cur {
        chain.push(r.range);
        cur = r.parent.as_deref();
    }
    assert_eq!(chain[0], range_of(&ws.main_text, "s)", 0).with_len(1));
    assert!(
        chain.contains(&range_of(&ws.main_text, "area(s)", 0)),
        "{chain:?}"
    );
    assert!(
        chain.contains(&range_of(&ws.main_text, "a = area(s)", 0)),
        "{chain:?}"
    );
    // Each holds the one before.
    for w in chain.windows(2) {
        assert!(
            w[1].start <= w[0].start && w[0].end <= w[1].end,
            "{chain:?}"
        );
    }
    c.shutdown();
}

#[test]
fn lsp_folding_document_highlights() {
    let ws = workspace(false);
    let mut c = ws.open();
    let r = c.request::<lsp::request::DocumentHighlightRequest>(pos(
        &ws.main,
        &ws.main_text,
        "a + shapes",
        0,
        0,
    ));
    let hs: Vec<lsp::DocumentHighlight> = serde_json::from_value(r).unwrap();
    assert_eq!(hs.len(), 2);
    assert_eq!(hs[0].kind, Some(lsp::DocumentHighlightKind::WRITE));
    assert_eq!(
        hs[0].range,
        range_of(&ws.main_text, "a = area", 0).with_len(1)
    );
    assert_eq!(hs[1].kind, Some(lsp::DocumentHighlightKind::READ));
    c.shutdown();
}

#[test]
fn lsp_folding_workspace_symbols() {
    let ws = workspace(false);
    let mut c = ws.open();
    let r = c.request::<lsp::request::WorkspaceSymbolRequest>(json!({ "query": "are" }));
    let symbols: Vec<lsp::SymbolInformation> = serde_json::from_value(r).unwrap();
    let area = symbols.iter().find(|s| s.name == "area").unwrap();
    assert_eq!(area.location.uri.as_str(), ws.shapes);
    assert_eq!(area.container_name.as_deref(), Some("app/shapes"));
    assert_eq!(area.kind, lsp::SymbolKind::FUNCTION);
    let r = c.request::<lsp::request::WorkspaceSymbolRequest>(json!({ "query": "Circ" }));
    let symbols: Vec<lsp::SymbolInformation> = serde_json::from_value(r).unwrap();
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "Circle" && s.kind == lsp::SymbolKind::CONSTRUCTOR)
    );
    // Not the names of dependencies.
    let r = c.request::<lsp::request::WorkspaceSymbolRequest>(json!({ "query": "shout" }));
    assert_eq!(r, json!([]));
    c.shutdown();
}

#[test]
fn lsp_folding_document_links() {
    let ws = workspace(false);
    let mut c = ws.open();
    let r = c.request::<lsp::request::DocumentLinkRequest>(
        json!({ "textDocument": { "uri": ws.main } }),
    );
    let links: Vec<lsp::DocumentLink> = serde_json::from_value(r).unwrap();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].range, range_of(&ws.main_text, "app/shapes", 0));
    assert_eq!(links[0].target.as_ref().unwrap().as_str(), ws.shapes);
    assert_eq!(links[1].target.as_ref().unwrap().as_str(), ws.lib);
    c.shutdown();
}

// ---- Diagnostics ----

#[test]
fn lsp_diagnostics_related_information_and_data() {
    let dir = tempfile::tempdir().unwrap();
    let text = "main = () { nope }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    let diags = c.diagnostics(&main);
    assert_eq!(diags[0].data, Some(json!({ "fix": "import" })));
    c.shutdown();
    // A problem in the code that a macro made: the template is related.
    let dir = tempfile::tempdir().unwrap();
    let main_text = "(m = app/macros)\nmain = () { m.bad() }\n";
    let macros = "(, (= syntax (/ i syntax)))\n(, bad)\n(@ macro (= bad (-> (,) (. syntax Syntax) `(+ 1 \"a\"))))\n";
    package(
        dir.path(),
        &[("src/main.i", main_text), ("src/macros.is", macros)],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let macros_uri = file_uri(&dir.path().join("src/macros.is"));
    let mut c = Client::start();
    c.open(&main, main_text);
    let diags = c.diagnostics(&main);
    let d = diags
        .iter()
        .find(|d| d.related_information.is_some())
        .unwrap_or_else(|| panic!("{diags:?}"));
    let related = d.related_information.as_ref().unwrap();
    assert_eq!(related[0].location.uri.as_str(), macros_uri);
    assert_eq!(related[0].message, "expanded from this template");
    c.shutdown();
}

#[test]
fn lsp_diagnostics_of_several_packages() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["one", "two"] {
        let p = dir.path().join(name);
        fs::create_dir_all(p.join("src")).unwrap();
        fs::write(p.join("i.toml"), format!("name = \"{name}\"\n")).unwrap();
        fs::write(p.join("src/main.i"), "main = () { nope }\n").unwrap();
    }
    let one = file_uri(&dir.path().join("one/src/main.i"));
    let two = file_uri(&dir.path().join("two/src/main.i"));
    let mut c = Client::start();
    c.open(&one, "main = () { nope }\n");
    c.open(&two, "main = () { nope }\n");
    assert_eq!(c.diagnostics(&one).len(), 1);
    assert_eq!(c.diagnostics(&two).len(), 1);
    c.change(&two, 2, "main = () { 1 }\n");
    assert_eq!(c.diagnostics(&two), vec![]);
    c.shutdown();
}

#[test]
fn lsp_diagnostics_cleared_when_a_new_document_is_closed() {
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", "main = () { 1 }\n")]);
    let path = dir.path().canonicalize().unwrap().join("src/new.i");
    let new = uri::from_path(&path).unwrap().as_str().to_string();
    let mut c = Client::start();
    c.open(&new, "y = nope\n");
    assert!(!c.diagnostics(&new).is_empty());
    c.notify::<lsp::notification::DidCloseTextDocument>(json!({ "textDocument": { "uri": new } }));
    assert_eq!(c.diagnostics(&new), vec![]);
    c.shutdown();
}

#[test]
fn lsp_diagnostics_cleared_when_a_file_is_deleted() {
    let dir = tempfile::tempdir().unwrap();
    package(
        dir.path(),
        &[
            ("src/main.i", "main = () { 1 }\n"),
            ("src/bad.i", "y = nope\n"),
        ],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let bad = file_uri(&dir.path().join("src/bad.i"));
    let mut c = Client::start();
    c.open(&main, "main = () { 1 }\n");
    assert!(!c.diagnostics(&bad).is_empty());
    fs::remove_file(dir.path().join("src/bad.i")).unwrap();
    c.notify::<lsp::notification::DidChangeWatchedFiles>(json!({
        "changes": [{ "uri": bad, "type": 3 }]
    }));
    assert_eq!(c.diagnostics(&bad), vec![]);
    c.shutdown();
}

#[test]
fn lsp_diagnostics_only_affected_packages_are_checked_again() {
    let ws = workspace(false);
    let mut c = ws.open();
    let lib_dir = ws.root.parent().unwrap().join("lib");
    let other = tempfile::tempdir().unwrap();
    package(other.path(), &[("src/main.i", "main = () { nope }\n")]);
    let other_main = file_uri(&other.path().join("src/main.i"));
    c.open(&other_main, "main = () { nope }\n");
    assert_eq!(c.diagnostics(&other_main).len(), 1);
    // A change of `app` checks `app` only.
    let text = ws.main_text.replace("\tn\n}", "\tn + 1\n}");
    c.change(&ws.main, 2, &text);
    c.diagnostics(&ws.main);
    std::thread::sleep(Duration::from_millis(500));
    assert!(!c.has_diagnostics(&other_main));
    // A change of `lib`, which `app` uses, checks `app` again.
    let lib = file_uri(&lib_dir.join("src/text.i"));
    c.open(
        &lib,
        &LIB_TEXT.replace("(\n\tshout\n\tTone(..)\n)", "(\n\tTone(..)\n)"),
    );
    let diags = c.diagnostics(&ws.main);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(lsp::NumberOrString::String("E0204".into()))),
        "{diags:?}"
    );
    assert!(!c.has_diagnostics(&other_main));
    c.shutdown();
}

#[test]
fn lsp_diagnostics_superseded_requests_are_cancelled() {
    let ws = workspace(false);
    let mut c = Client::with_debounce(Duration::from_secs(600));
    c.open(&ws.main, &ws.main_text);
    c.diagnostics(&ws.main);
    let text = format!("{}extra = 1\n", ws.main_text);
    c.change(&ws.main, 2, &text);
    // Two requests at the changed place: the first waits for a check and
    // is superseded by the second.
    let first = RequestId::from(1000);
    let second = RequestId::from(1001);
    for id in [&first, &second] {
        c.conn
            .sender
            .send(Message::Request(Request::new(
                id.clone(),
                lsp::request::HoverRequest::METHOD.to_string(),
                pos(&ws.main, &text, "extra", 0, 1),
            )))
            .unwrap();
    }
    let mut answers = HashMapIds::new();
    while answers.len() < 2 {
        match c.conn.receiver.recv_timeout(TIMEOUT).expect("no response") {
            Message::Response(r) => {
                answers.insert(r.id.clone(), r);
            }
            Message::Notification(n) => c.pending.push(n),
            _ => {}
        }
    }
    let r1 = &answers[&first];
    let r2 = &answers[&second];
    assert_eq!(
        r1.response_result.as_ref().err().map(|e| e.code),
        Some(-32801),
        "{r1:?}"
    );
    assert!(
        r2.response_result.as_ref().unwrap()["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("extra: 1")
    );
    c.shutdown();
}

type HashMapIds = std::collections::HashMap<RequestId, lsp_server::Response>;

#[test]
fn lsp_diagnostics_lisp_surface() {
    let ws = workspace(true);
    let mut c = ws.open();
    let broken = to_lisp(&APP_MAIN.replace("\tn\n}", "\tnope\n}"));
    c.change(&ws.main, 2, &broken);
    let diags = c.diagnostics(&ws.main);
    let d = diags
        .iter()
        .find(|d| d.code == Some(lsp::NumberOrString::String("E0201".into())))
        .unwrap();
    assert_eq!(d.range, range_of(&broken, "nope", 0));
    c.shutdown();
}

// ---- Every request at every offset ----

/// Every request at every character of a text: each gets a response that
/// is a result or an error the server means to give, never a lost
/// request.
fn every_request_everywhere(c: &mut Client, uri: &str, text: &str) {
    use lsp::request::*;
    let ix = ilang_lsp::position::LineIndex::new(text);
    let end = ix.position(text.len() as u32);
    let whole = json!({ "start": { "line": 0, "character": 0 }, "end": end });
    let doc = json!({ "uri": uri });
    for (method, p) in [
        (
            DocumentSymbolRequest::METHOD,
            json!({ "textDocument": doc }),
        ),
        (FoldingRangeRequest::METHOD, json!({ "textDocument": doc })),
        (DocumentLinkRequest::METHOD, json!({ "textDocument": doc })),
        (
            InlayHintRequest::METHOD,
            json!({ "textDocument": doc, "range": whole }),
        ),
        (
            SemanticTokensFullRequest::METHOD,
            json!({ "textDocument": doc }),
        ),
        (
            Formatting::METHOD,
            json!({ "textDocument": doc, "options": { "tabSize": 4, "insertSpaces": false } }),
        ),
    ] {
        raw_request(c, method, p);
    }
    for (i, _) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        let p = ix.position(i as u32);
        let at = json!({ "textDocument": doc, "position": p });
        let range = json!({ "start": p, "end": p });
        let mut with_context = at.clone();
        with_context["context"] = json!({ "includeDeclaration": true });
        let mut renamed = at.clone();
        renamed["newName"] = json!("fresh_name");
        for (method, params) in [
            (HoverRequest::METHOD, at.clone()),
            (GotoDefinition::METHOD, at.clone()),
            (GotoTypeDefinition::METHOD, at.clone()),
            (GotoImplementation::METHOD, at.clone()),
            (References::METHOD, with_context),
            (DocumentHighlightRequest::METHOD, at.clone()),
            (Completion::METHOD, at.clone()),
            (SignatureHelpRequest::METHOD, at.clone()),
            (PrepareRenameRequest::METHOD, at.clone()),
            (Rename::METHOD, renamed),
            (
                SelectionRangeRequest::METHOD,
                json!({ "textDocument": doc, "positions": [p] }),
            ),
            (
                CodeActionRequest::METHOD,
                json!({ "textDocument": doc, "range": range, "context": { "diagnostics": [] } }),
            ),
        ] {
            raw_request(c, method, params);
        }
    }
}

fn raw_request(c: &mut Client, method: &str, params: Value) {
    c.next_id += 1;
    let id = RequestId::from(c.next_id);
    c.conn
        .sender
        .send(Message::Request(Request::new(
            id.clone(),
            method.to_string(),
            params.clone(),
        )))
        .unwrap();
    loop {
        match c
            .conn
            .receiver
            .recv_timeout(TIMEOUT)
            .unwrap_or_else(|_| panic!("no response to {method} {params}"))
        {
            Message::Response(r) if r.id == id => {
                if let Err(e) = &r.response_result {
                    // Refused renames, and nothing else.
                    assert!(
                        e.code == lsp_server::ErrorCode::RequestFailed as i32
                            || e.code == lsp_server::ErrorCode::InvalidParams as i32
                                && method == lsp::request::Rename::METHOD,
                        "{method} {params}: {e:?}"
                    );
                }
                return;
            }
            Message::Notification(n) => c.pending.push(n),
            _ => {}
        }
    }
}

const BROKEN: &str = /* old-syntax-check: E0010,E0101,E0010,E0010,E0101,E0010,E0002 */
    "(\n\tutil = app/util\n)\n\nShape { Circle(Int) Rect(\n\nf = (s: Shape, ) {\n\ts |> {\n\t\tCircle(r) -> r +\n\t}\n\tutil.\n}\n\nmain = () { f(Circle(1) \"x\n";

#[test]
fn lsp_every_offset_of_correct_files() {
    let ws = workspace(false);
    let mut c = ws.open();
    every_request_everywhere(&mut c, &ws.main, &ws.main_text);
    c.open(&ws.shapes, SHAPES);
    every_request_everywhere(&mut c, &ws.shapes, SHAPES);
    c.shutdown();
}

#[test]
fn lsp_every_offset_of_lisp_files() {
    let ws = workspace(true);
    let mut c = ws.open();
    every_request_everywhere(&mut c, &ws.main, &ws.main_text);
    c.shutdown();
}

#[test]
fn lsp_every_offset_of_broken_files() {
    let dir = tempfile::tempdir().unwrap();
    let util = "(double)\n\ndouble = (n: Int) -> Int { n * 2 }\n";
    package(
        dir.path(),
        &[
            ("src/main.i", BROKEN),
            ("src/util.i", util),
            ("src/lisp.is", "(= f (-> (x) _ {(+ x\n"),
        ],
    );
    let main = file_uri(&dir.path().join("src/main.i"));
    let lisp = file_uri(&dir.path().join("src/lisp.is"));
    let mut c = Client::start();
    c.open(&main, BROKEN);
    assert!(!c.diagnostics(&main).is_empty());
    every_request_everywhere(&mut c, &main, BROKEN);
    c.open(&lisp, "(= f (-> (x) _ {(+ x\n");
    every_request_everywhere(&mut c, &lisp, "(= f (-> (x) _ {(+ x\n");
    c.shutdown();
}

// ---- Latency on `std` ----

#[test]
fn lsp_latency_on_std() {
    let std_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let list = std_dir.join("src/list.i");
    let text = fs::read_to_string(&list).unwrap();
    let uri = file_uri(&list);
    let mut c = Client::start();
    c.open(&uri, &text);
    let started = Instant::now();
    c.diagnostics_within(&uri, Duration::from_secs(900));
    eprintln!("first check of std: {:?}", started.elapsed());
    // The first requests after the check, and then the next ones.
    let at = |needle: &str| {
        let (l, ch) = find(&text, needle, 0);
        let (l, ch) = if needle.starts_with('\n') {
            (l + 1, 1)
        } else {
            (l, ch + 4)
        };
        position_params(&uri, l, ch)
    };
    let mut times = Vec::new();
    for round in 0..3 {
        for (name, p) in [
            ("hover", at("\nmap = <A")),
            ("definition", at("{ reverse(acc)")),
            ("completion", at("{ reverse(acc)")),
        ] {
            let started = Instant::now();
            let cpu = c.server_cpu();
            let r = match name {
                "hover" => c.request::<lsp::request::HoverRequest>(p),
                "definition" => c.request::<lsp::request::GotoDefinition>(p),
                _ => c.request::<lsp::request::Completion>(p),
            };
            let took = c.server_cpu() - cpu;
            assert!(!r.is_null(), "{name}");
            times.push((round, name, took, started.elapsed()));
        }
    }
    // The bound is on the CPU time of the server's thread, which answers
    // the requests: the wall time also counts the other tests and workers
    // that share the machine.
    eprintln!("CPU time and wall time of requests on std/list: {times:?}");
    for (round, name, took, _) in &times {
        assert!(
            *took < Duration::from_millis(200),
            "{name} took {took:?} of CPU time in round {round}"
        );
    }
    c.shutdown();
}

#[test]
fn lsp_capabilities_are_advertised() {
    let (server, client) = Connection::memory();
    let handle = std::thread::spawn(move || {
        ilang_lsp::run(
            &server,
            Options {
                debounce: Duration::from_millis(20),
                offline: true,
            },
        )
        .unwrap();
    });
    let mut c = Client {
        conn: client,
        server: Some(handle),
        next_id: 0,
        pending: Vec::new(),
        server_thread: None,
    };
    let init = c.request::<lsp::request::Initialize>(json!({ "capabilities": {} }));
    let caps = &init["capabilities"];
    for key in [
        "referencesProvider",
        "documentHighlightProvider",
        "workspaceSymbolProvider",
        "typeDefinitionProvider",
        "implementationProvider",
        "inlayHintProvider",
        "foldingRangeProvider",
        "selectionRangeProvider",
    ] {
        assert_eq!(caps[key], json!(true), "{key}");
    }
    assert_eq!(caps["renameProvider"]["prepareProvider"], json!(true));
    assert_eq!(caps["completionProvider"]["resolveProvider"], json!(true));
    assert_eq!(
        caps["signatureHelpProvider"]["triggerCharacters"],
        json!(["(", ","])
    );
    assert_eq!(
        caps["codeActionProvider"]["codeActionKinds"],
        json!(["quickfix", "refactor.rewrite"])
    );
    assert_eq!(
        caps["documentLinkProvider"]["resolveProvider"],
        json!(false)
    );
    c.notify::<lsp::notification::Initialized>(json!({}));
    c.shutdown();
}

const MATCH: &str = "Circle = (Int)\nRect = (width = Int, height = Int)\nDot = ()\nShape = Circle | Rect | Dot\n\nf = (s: Shape) { s |> (Dot) { 0 } | (Circle(1)) { 1 } }\n\nmain = () { f(Dot) }\n";

#[test]
fn lsp_code_action_missing_clauses_lisp_surface() {
    let dir = tempfile::tempdir().unwrap();
    let text = to_lisp(MATCH);
    package(dir.path(), &[("src/main.is", &text)]);
    let main = file_uri(&dir.path().join("src/main.is"));
    let mut c = Client::start();
    c.open(&main, &text);
    let diags = c.diagnostics(&main);
    let d = diags
        .iter()
        .find(|d| d.code == Some(lsp::NumberOrString::String("E0516".into())))
        .unwrap_or_else(|| panic!("{diags:?}\n{text}"));
    let actions = code_actions(&mut c, &main, d.range);
    let after = applied(action(&actions, "Add the missing clauses"), &text, &main);
    assert!(after.contains("(panic \"todo\")"), "{after}");
    c.change(&main, 2, &after);
    let diags = c.diagnostics(&main);
    assert!(
        !diags
            .iter()
            .any(|d| d.severity == Some(lsp::DiagnosticSeverity::ERROR)),
        "{diags:?}\n{after}"
    );
    c.shutdown();
}

#[test]
fn lsp_completion_constructors_lisp_surface() {
    let dir = tempfile::tempdir().unwrap();
    let text = to_lisp(MATCH);
    package(dir.path(), &[("src/main.is", &text)]);
    let main = file_uri(&dir.path().join("src/main.is"));
    let mut c = Client::start();
    c.open(&main, &text);
    c.diagnostics(&main);
    // Before the `}` that ends the clauses.
    let close = text.find("_ 1)").unwrap_or_else(|| panic!("{text}")) + 4;
    let ix = ilang_lsp::position::LineIndex::new(&text);
    let p = ix.position(close as u32);
    let r = c.request::<lsp::request::Completion>(position_params(&main, p.line, p.character));
    let items: Vec<lsp::CompletionItem> = serde_json::from_value(r).unwrap();
    let circle = item(&items, "Circle");
    let insert = circle.insert_text.clone().unwrap();
    assert!(
        insert.contains("Circle") && insert.contains("${1:_}"),
        "{insert}"
    );
    assert_eq!(items[0].label, "Circle");
    c.shutdown();
}

#[test]
fn formats_lisp_documents_with_the_source_formatter() {
    let dir = tempfile::tempdir().unwrap();
    let source = "// keep\n(= main (-> (,) _ {(= first 1) (= second 2) (= third 3) (+ first (+ second third))}))\n";
    package(dir.path(), &[("src/main.is", source)]);
    let main = file_uri(&dir.path().join("src/main.is"));
    let mut client = Client::start();
    client.open(&main, source);
    let edits = client.request::<lsp::request::Formatting>(json!({
        "textDocument": { "uri": main },
        "options": { "tabSize": 4, "insertSpaces": false }
    }));
    let expected = ilang_syntax::lisp::format(source).unwrap();
    assert!(expected.lines().count() >= 3);
    assert_eq!(edits[0]["newText"], expected);
    client.change(&main, 2, &expected);
    let edits = client.request::<lsp::request::Formatting>(json!({
        "textDocument": { "uri": main },
        "options": { "tabSize": 4, "insertSpaces": false }
    }));
    assert!(edits.is_null());
    client.change(&main, 3, "(= broken");
    let edits = client.request::<lsp::request::Formatting>(json!({
        "textDocument": { "uri": main },
        "options": { "tabSize": 4, "insertSpaces": false }
    }));
    assert!(edits.is_null());
    client.shutdown();
}

#[test]
fn core_builtin_type_hover_and_definition() {
    let dir = tempfile::tempdir().unwrap();
    let text = "(core = i/core)\nf = (x: core.Int) -> Int { x }\nmain = () { Nil }\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    let _ = c.request::<lsp::request::HoverRequest>(position_params(&main, 1, 14));
    let _ = c.request::<lsp::request::GotoDefinition>(position_params(&main, 1, 14));
    c.shutdown();
}
