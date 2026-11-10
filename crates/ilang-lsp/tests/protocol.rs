//! The server over an in-memory connection: the messages of an editor
//! session, and the answers.

use ilang_lsp::{Options, uri};
use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types as lsp;
use lsp_types::notification::Notification as _;
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
}

const TIMEOUT: Duration = Duration::from_secs(30);

impl Client {
    fn start() -> Client {
        Client::with_debounce(Duration::from_millis(20))
    }

    fn with_debounce(debounce: Duration) -> Client {
        let (server, client) = Connection::memory();
        let handle = std::thread::spawn(move || {
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
        let is_it = |n: &Notification| {
            n.method == lsp::notification::PublishDiagnostics::METHOD && n.params["uri"] == uri
        };
        let n = match self.pending.iter().position(is_it) {
            Some(i) => self.pending.remove(i),
            None => loop {
                match self
                    .conn
                    .receiver
                    .recv_timeout(TIMEOUT)
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
    let (i, _) = text.match_indices(needle).nth(nth).unwrap();
    let before = &text[..i];
    let line = before.matches('\n').count() as u32;
    let col = before[before.rfind('\n').map_or(0, |n| n + 1)..]
        .encode_utf16()
        .count() as u32;
    (line, col)
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

const MAIN: &str = " (util = app/util)\n\n/// Adds two numbers.\nadd = (x, y) { x + y }\n\nUser { User(name: String, age: Int) }\n\ngreet = (u: User) { \"héllo 𝕏 \" <> u.name }\n\nmain = () { util.double(add(1, 2)) }\n";

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
    assert_eq!(c.diagnostics(&main), vec![]);

    // Hover shows the type, with the documentation.
    let (l, ch) = find(MAIN, "add(1", 0);
    let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch + 1));
    let value = h["contents"]["value"].as_str().unwrap();
    assert!(value.contains("add: (x: Int, y: Int) -> Int"), "{value}");
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
    let (dl, dch) = find(MAIN, "User {", 0);
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
    assert_eq!(c.diagnostics(&main), vec![]);

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
    assert_eq!(r[0]["detail"], "(n: Int) -> Int");

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
    assert_eq!(c.diagnostics(&main), vec![]);
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
    assert_eq!(c.diagnostics(&main), vec![]);
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
    assert_eq!(c.diagnostics(&new), vec![]);
    c.shutdown();
}

/// Completion after a dot that follows an expression that is not a name.
#[test]
fn completion_after_an_expression() {
    let dir = tempfile::tempdir().unwrap();
    let text = "Point { Point(x: Int, y: Int) }\n\
                make = () -> Point { Point(1, 2) }\n\
                f = (t: Point) {\n  a = make().x\n  b = { t }.y\n  a + b\n}\n";
    package(dir.path(), &[("src/main.i", text)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, text);
    assert_eq!(c.diagnostics(&main), vec![]);
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
    assert_eq!(c.diagnostics(&main), vec![]);
    let started = Instant::now();
    // A line before everything.
    let edited = format!("\n{MAIN}");
    c.change(&main, 2, &edited);
    let (l, ch) = find(&edited, "add(1", 0);
    let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch + 1));
    let value = h["contents"]["value"].as_str().unwrap();
    assert!(value.contains("add: (x: Int, y: Int) -> Int"), "{value}");
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
    assert_eq!(c.diagnostics(&main), vec![]);
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
    assert_eq!(c.diagnostics(&main), vec![]);
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
    assert_eq!(c.diagnostics(&main), vec![]);
    let mut hover = |needle: &str| {
        let (l, ch) = find(text, needle, 0);
        let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch));
        let value = h["contents"]["value"].as_str().unwrap();
        value
            .trim_start_matches("```i\n")
            .trim_end_matches("\n```")
            .to_string()
    };
    assert_eq!(hover("k ="), "k: (f: (A) -> B ? E ! N, x: A) -> B ? E ! N");
    assert_eq!(hover("y ="), "y: B");
    assert_eq!(hover("f, x"), "f: (A) -> B ? E ! N");
    assert_eq!(hover("g) {"), "g: (B) -> C ? E2 ! N2");
    assert_eq!(hover("f, g"), "f: (A) -> B ? E ! N");
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
    assert_eq!(c.diagnostics(&main), vec![]);
    let mut hover = |needle: &str| {
        let (l, ch) = find(text, needle, 0);
        let h = c.request::<lsp::request::HoverRequest>(position_params(&main, l, ch));
        let value = h["contents"]["value"].as_str().unwrap();
        value
            .trim_start_matches("```i\n")
            .trim_end_matches("\n```")
            .to_string()
    };
    assert_eq!(hover("one"), "one: (x: A) -> 1");
    assert_eq!(hover("x) {\n  y = x"), "x: A");
    assert_eq!(hover("y = x"), "y: A");
    assert_eq!(
        hover("drop2"),
        "drop2: (x: A, g: (1) -> B ? E ! N, h: C) -> B ? E ! N"
    );
    assert_eq!(hover("x, g, h"), "x: A");
    assert_eq!(hover("h) {"), "h: C");
    assert_eq!(hover("y = (x, h)"), "y: (A, C)");
    assert_eq!(
        hover("effonly"),
        "effonly: (f: (A) -> B ? E ! N, x: A) -> Nil ? E ! N"
    );
    assert_eq!(hover("f, x) {\n  r"), "f: (A) -> B ? E ! N");
    assert_eq!(hover("r = "), "r: B");
    c.shutdown();
}

#[test]
fn t2_nested_pattern_heads_support_hover_and_definition_in_recovery() {
    // X4: every pattern head targets its existing type, at every depth.
    let source = include_str!("../../ilang-check/tests/value_type_review/nested/src/main.i")
        .replace("x: Item = 2", "x: Item = 1");
    let source = source.as_str();
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", source)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, source);
    assert!(c.diagnostics(&main).is_empty());
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
    // W520/§6.9 keeps the comparison span and shows both types.
    let source = "x = 1 == 2\n";
    let dir = tempfile::tempdir().unwrap();
    package(dir.path(), &[("src/main.i", source)]);
    let main = file_uri(&dir.path().join("src/main.i"));
    let mut c = Client::start();
    c.open(&main, source);
    let ds = c.diagnostics(&main);
    assert_eq!(ds.len(), 1, "{ds:?}");
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
