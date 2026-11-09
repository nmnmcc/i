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
use std::time::Duration;

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
        let (server, client) = Connection::memory();
        let handle = std::thread::spawn(move || {
            let options = Options {
                debounce: Duration::from_millis(20),
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

const MAIN: &str = "import (util = app/util)

/// Adds two numbers.
add = (x, y) { x + y }

User { User(name: String, age: Int) }

greet = (u: User) { \"héllo 𝕏 \" <> u.name }

main = () { util.double(add(1, 2)) }
";

const UTIL: &str = "export (double)

double = (n: Int) -> Int { n * 2 }
";

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
    for name in [
        "x", "y", "add", "greet", "util", "User", "Int", "panic", "case",
    ] {
        assert!(labels.contains(&name), "{name} in {labels:?}");
    }

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

    // Formatting is not in this build.
    if ilang_lsp::format::formatter().is_none() {
        let e = c.try_request::<lsp::request::Formatting>(json!({
            "textDocument": { "uri": main },
            "options": { "tabSize": 2, "insertSpaces": true }
        }));
        assert!(e.unwrap_err().contains("not available"));
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
    c.open(&util, &UTIL.replace("export (double)", "export ()"));
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
