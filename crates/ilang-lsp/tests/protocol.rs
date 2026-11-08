//! Tests exercise the real language server over JSON-RPC, including unsaved
//! documents. They deliberately use no compiler-specific testing hooks.

use lsp_server::{Connection, Message};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(30);

struct Project {
    directory: TempDir,
}

impl Project {
    fn new(modules: &[(&str, &str)]) -> Self {
        let directory = tempfile::tempdir().expect("create project directory");
        fs::write(
            directory.path().join("i.toml"),
            "name = \"protocol_test\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n",
        )
        .expect("write project manifest");
        fs::create_dir(directory.path().join("src")).expect("create source directory");
        let project = Self { directory };
        for (module, source) in modules {
            project.write(module, source);
        }
        project
    }

    fn path(&self, module: &str) -> PathBuf {
        self.directory
            .path()
            .join("src")
            .join(format!("{module}.i"))
    }

    fn write(&self, module: &str, source: &str) {
        let path = self.path(module);
        fs::create_dir_all(path.parent().expect("source parent")).expect("create module directory");
        fs::write(path, source).expect("write module");
    }

    fn uri(&self, module: &str) -> String {
        file_uri(&self.path(module))
    }
}

// Temporary directory names contain only URI-safe characters on supported hosts.
// Percent-encoding the few reserved characters also makes a relocated TMPDIR safe.
fn file_uri(path: &Path) -> String {
    let path = path
        .to_str()
        .expect("UTF-8 fixture path")
        .replace('\\', "/");
    let mut encoded = String::new();
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b':' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

struct Client {
    connection: Option<Connection>,
    finished: mpsc::Receiver<Result<(), String>>,
    thread: Option<thread::JoinHandle<()>>,
    next_id: i32,
    diagnostics: HashMap<String, Vec<Value>>,
}

impl Client {
    fn start(project: &Project) -> Self {
        let (connection, server) = Connection::memory();
        let (finished_tx, finished) = mpsc::channel();
        let thread = thread::spawn(move || {
            let outcome = ilang_lsp::run_connection(server).map_err(|error| error.to_string());
            let _ = finished_tx.send(outcome);
        });
        let mut client = Self {
            connection: Some(connection),
            finished,
            thread: Some(thread),
            next_id: 1,
            diagnostics: HashMap::new(),
        };
        let initialization = client.request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": file_uri(project.directory.path()),
                "clientInfo": { "name": "ilang-protocol-tests" },
                "capabilities": {},
            }),
        );
        assert_eq!(initialization["capabilities"]["hoverProvider"], true);
        assert_eq!(initialization["capabilities"]["definitionProvider"], true);
        assert_eq!(
            initialization["capabilities"]["textDocumentSync"]["change"], 1,
            "the client sends the advertised full-document changes"
        );
        client.notify("initialized", json!({}));
        client
    }

    fn send(&self, value: Value) {
        let message: Message = serde_json::from_value(value).expect("valid JSON-RPC message");
        self.connection
            .as_ref()
            .expect("connected client")
            .sender
            .send(message)
            .expect("send to language server");
    }

    fn notify(&self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "language server timed out answering {method}"
            );
            let message = self
                .connection
                .as_ref()
                .expect("connected client")
                .receiver
                .recv_timeout(remaining)
                .unwrap_or_else(|error| panic!("language server did not answer {method}: {error}"));
            let value = serde_json::to_value(message).expect("serialize server message");
            if value["id"] == id && value.get("method").is_none() {
                assert!(
                    value.get("error").is_none(),
                    "{method} failed: {}",
                    value["error"]
                );
                return value["result"].clone();
            }
            if value.get("id").is_some() && value.get("method").is_some() {
                assert_ne!(
                    value["method"], "window/workDoneProgress/create",
                    "the client did not advertise progress support"
                );
                // Acknowledge other server requests so they cannot deadlock tests.
                self.send(json!({ "jsonrpc": "2.0", "id": value["id"], "result": null }));
            } else if value["method"] == "textDocument/publishDiagnostics" {
                let parameters = &value["params"];
                self.diagnostics.insert(
                    parameters["uri"]
                        .as_str()
                        .expect("diagnostic URI")
                        .to_owned(),
                    parameters["diagnostics"]
                        .as_array()
                        .expect("diagnostics")
                        .clone(),
                );
            }
        }
    }

    fn open(&self, uri: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "i", "version": 1, "text": text } }),
        );
    }

    fn change(&self, uri: &str, version: i32, text: &str) {
        self.notify(
            "textDocument/didChange",
            json!({ "textDocument": { "uri": uri, "version": version }, "contentChanges": [{ "text": text }] }),
        );
    }

    fn at(&mut self, method: &str, uri: &str, position: Value) -> Value {
        self.request(
            method,
            json!({ "textDocument": { "uri": uri }, "position": position }),
        )
    }

    fn errors(&self, uri: &str) -> Vec<&Value> {
        self.diagnostics
            .get(uri)
            .into_iter()
            .flatten()
            .filter(|diagnostic| diagnostic["severity"] == 1)
            .collect()
    }

    fn finish(mut self) {
        self.request("shutdown", Value::Null);
        self.notify("exit", Value::Null);
        self.connection.take();
        self.finished
            .recv_timeout(TIMEOUT)
            .expect("language server terminates after shutdown")
            .expect("language server exits cleanly");
        self.thread
            .take()
            .expect("server thread")
            .join()
            .expect("server does not panic");
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Closing the channels unblocks the server even when an assertion failed.
        // Do not join here: a failing test must never become an unbounded wait.
        self.connection.take();
    }
}

fn position(source: &str, token: &str) -> Value {
    let offset = source.find(token).expect("fixture token exists");
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count();
    let character = before
        .rsplit('\n')
        .next()
        .expect("current line")
        .encode_utf16()
        .count();
    json!({ "line": line, "character": character })
}

#[test]
fn semantic_navigation_and_rename_across_modules() {
    let helper = "pub fn double(value: Int) -> Int {\n  value * 2\n}\n";
    let source = "import helper\n\npub fn main() -> Int {\n  helper.double(21)\n}\n";
    let project = Project::new(&[("app", source), ("helper", helper)]);
    let uri = project.uri("app");
    let helper_uri = project.uri("helper");
    let mut client = Client::start(&project);
    client.open(&uri, source);

    let symbol = position(source, "double");
    let hover = client.at("textDocument/hover", &uri, symbol.clone());
    let hover_text = hover["contents"].to_string();
    assert!(
        hover_text.contains("fn(Int) -> Int"),
        "inferred hover: {hover}"
    );
    assert!(
        client.errors(&uri).is_empty(),
        "valid module diagnostics: {:?}",
        client.errors(&uri)
    );

    let definition = client.at("textDocument/definition", &uri, symbol.clone());
    assert_eq!(definition["uri"], helper_uri);
    assert_eq!(definition["range"]["start"]["line"], 0);
    assert_eq!(definition["range"]["start"]["character"], 0);

    let completion = client.at("textDocument/completion", &uri, symbol.clone());
    let items = completion
        .as_array()
        .or_else(|| completion["items"].as_array())
        .expect("completion items");
    assert!(
        items.iter().any(|item| item["label"] == "helper.double"),
        "completion: {completion}"
    );

    let references = client.request(
        "textDocument/references",
        json!({ "textDocument": { "uri": uri }, "position": symbol, "context": { "includeDeclaration": true } }),
    );
    let references = references.as_array().expect("reference locations");
    assert!(references.iter().any(|location| location["uri"] == uri));
    assert!(
        references
            .iter()
            .any(|location| location["uri"] == helper_uri)
    );

    let rename = client.request(
        "textDocument/rename",
        json!({ "textDocument": { "uri": uri }, "position": position(source, "double"), "newName": "twice" }),
    );
    for affected in [&uri, &helper_uri] {
        let edits = rename["changes"][affected]
            .as_array()
            .expect("cross-module rename edits");
        assert!(
            edits.iter().any(|edit| edit["newText"] == "twice"),
            "rename: {rename}"
        );
    }
    client.finish();
}

#[test]
fn diagnostics_follow_unsaved_changes_and_saved_disk_content() {
    let broken = "pub fn main() -> Int { \"wrong\" }\n";
    let fixed = "pub fn main() -> Int { 42 }\n";
    let project = Project::new(&[("app", broken)]);
    let uri = project.uri("app");
    let mut client = Client::start(&project);
    client.open(&uri, broken);
    client.request(
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": uri } }),
    );
    assert!(
        !client.errors(&uri).is_empty(),
        "type mismatch must publish a diagnostic"
    );
    assert!(client.errors(&uri).iter().any(|diagnostic| {
        let message = diagnostic["message"].as_str().unwrap_or_default();
        message.contains("Int") && message.contains("String")
    }));

    client.change(&uri, 2, fixed);
    let hover = client.at("textDocument/hover", &uri, position(fixed, "42"));
    assert!(
        hover["contents"].to_string().contains("Int"),
        "unsaved hover: {hover}"
    );
    assert!(
        client.errors(&uri).is_empty(),
        "fixed in-memory document clears old diagnostics"
    );
    assert_eq!(
        fs::read_to_string(project.path("app")).expect("disk source"),
        broken
    );

    client.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": uri } }),
    );
    client.request(
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": uri } }),
    );
    assert!(
        !client.errors(&uri).is_empty(),
        "save invalidates the in-memory overlay and reads disk"
    );

    project.write("app", fixed);
    client.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": uri } }),
    );
    client.request(
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": uri } }),
    );
    assert!(
        client.errors(&uri).is_empty(),
        "saved correction clears diagnostics"
    );
    client.finish();
}

fn byte_position(source: &str, position: &Value) -> usize {
    let line =
        usize::try_from(position["line"].as_u64().expect("line number")).expect("line fits usize");
    let utf16_column = usize::try_from(position["character"].as_u64().expect("column"))
        .expect("column fits usize");
    let line_start = source
        .split_inclusive('\n')
        .take(line)
        .map(str::len)
        .sum::<usize>();
    let mut units = 0;
    for (offset, character) in source[line_start..].char_indices() {
        if units == utf16_column {
            return line_start + offset;
        }
        units += character.len_utf16();
    }
    assert_eq!(units, utf16_column);
    source.len()
}

fn apply_edits(source: &str, edits: &Value) -> String {
    let mut edits = edits
        .as_array()
        .expect("text edits")
        .iter()
        .map(|edit| {
            (
                byte_position(source, &edit["range"]["start"]),
                byte_position(source, &edit["range"]["end"]),
                edit["newText"].as_str().expect("replacement text"),
            )
        })
        .collect::<Vec<_>>();
    edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut output = source.to_owned();
    for (start, end, text) in edits {
        output.replace_range(start..end, text);
    }
    output
}

#[test]
fn formatting_is_idempotent_and_positions_are_utf16() {
    let source = "pub fn main(){let answer=40+2 answer}\n";
    let project = Project::new(&[("app", source)]);
    let uri = project.uri("app");
    let mut client = Client::start(&project);
    client.open(&uri, source);
    let formatting_parameters = json!({ "textDocument": { "uri": uri }, "options": { "tabSize": 2, "insertSpaces": true } });
    let edits = client.request("textDocument/formatting", formatting_parameters.clone());
    let formatted = apply_edits(source, &edits);
    assert_eq!(
        formatted,
        "pub fn main() {\n  let answer = 40 + 2\n  answer\n}\n"
    );
    client.change(&uri, 2, &formatted);
    let edits = client.request("textDocument/formatting", formatting_parameters);
    assert_eq!(apply_edits(&formatted, &edits), formatted);

    let unicode = "pub fn main() { let emoji = \"🦀\" #(emoji, 42) }\n";
    client.change(&uri, 3, unicode);
    let expected = position(unicode, "42");
    let hover = client.at("textDocument/hover", &uri, expected.clone());
    assert!(
        hover["contents"].to_string().contains("Int"),
        "hover after surrogate pair: {hover}"
    );
    assert_eq!(hover["range"]["start"], expected);
    client.finish();
}

#[test]
fn local_path_dependency_navigation_uses_the_dependency_source() {
    let source = "import arithmetic\n\npub fn main() { arithmetic.answer() }\n";
    let project = Project::new(&[("app", source)]);
    let dependency = project.directory.path().join("packages/arithmetic");
    fs::create_dir_all(dependency.join("src")).expect("create local dependency");
    fs::write(
        dependency.join("i.toml"),
        "name = \"arithmetic\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n",
    )
    .expect("write dependency manifest");
    let dependency_source = dependency.join("src/arithmetic.i");
    fs::write(&dependency_source, "pub fn answer() -> Int { 42 }\n")
        .expect("write dependency source");
    fs::write(
        project.directory.path().join("i.toml"),
        "name = \"protocol_test\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n\n[dependencies]\narithmetic = { path = \"./packages/arithmetic\" }\n",
    )
    .expect("register local dependency");
    let uri = project.uri("app");
    let mut client = Client::start(&project);
    client.open(&uri, source);
    let symbol = position(source, "answer");
    let hover = client.at("textDocument/hover", &uri, symbol.clone());
    assert!(hover["contents"].to_string().contains("fn() -> Int"));
    let definition = client.at("textDocument/definition", &uri, symbol);
    assert_eq!(definition["uri"], file_uri(&dependency_source));
    assert!(client.errors(&uri).is_empty());
    client.finish();
}

#[test]
fn unsaved_new_modules_participate_in_inference_and_imports() {
    let source = "import draft\n\npub fn main() { draft.add(41) }\n";
    let draft = "pub fn add(value) { value + 1 }\n";
    let project = Project::new(&[("app", source)]);
    let uri = project.uri("app");
    let draft_uri = project.uri("draft");
    assert!(!project.path("draft").exists());
    let mut client = Client::start(&project);
    client.open(&draft_uri, draft);
    client.open(&uri, source);

    let argument_hover = client.at("textDocument/hover", &draft_uri, position(draft, "value"));
    assert!(
        argument_hover["contents"].to_string().contains("Int"),
        "inferred unsaved argument hover: {argument_hover}"
    );
    let symbol = position(source, "add");
    let imported_hover = client.at("textDocument/hover", &uri, symbol.clone());
    assert!(
        imported_hover["contents"]
            .to_string()
            .contains("fn(Int) -> Int"),
        "imported unsaved module hover: {imported_hover}"
    );
    let definition = client.at("textDocument/definition", &uri, symbol.clone());
    assert_eq!(definition["uri"], draft_uri);
    let completion = client.at("textDocument/completion", &uri, symbol);
    let items = completion
        .as_array()
        .or_else(|| completion["items"].as_array())
        .expect("completion items");
    assert!(
        items.iter().any(|item| item["label"] == "draft.add"),
        "unsaved module completion: {completion}"
    );
    assert!(client.errors(&uri).is_empty());
    assert!(client.errors(&draft_uri).is_empty());
    assert!(
        !project.path("draft").exists(),
        "LSP must not persist the editor's unsaved source"
    );
    client.finish();
}

#[test]
fn manifest_content_change_reloads_dependencies_with_unchanged_mtime() {
    let source = "import arithmetic\n\npub fn main() { arithmetic.answer() }\n";
    let project = Project::new(&[("app", source)]);
    for (name, result) in [("first", "42"), ("other", "\"forty-two\"")] {
        let dependency = project.directory.path().join("packages").join(name);
        fs::create_dir_all(dependency.join("src")).expect("create local dependency");
        fs::write(
            dependency.join("i.toml"),
            "name = \"arithmetic\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n",
        )
        .expect("write dependency manifest");
        fs::write(
            dependency.join("src/arithmetic.i"),
            format!("pub fn answer() {{ {result} }}\n"),
        )
        .expect("write dependency source");
    }
    let manifest = project.directory.path().join("i.toml");
    let configuration = |dependency| {
        format!(
            "name = \"protocol_test\"\nversion = \"1.0.0\"\ntarget = \"wasm\"\n\n[dependencies]\narithmetic = {{ path = \"./packages/{dependency}\" }}\n"
        )
    };
    fs::write(&manifest, configuration("first")).expect("register initial dependency");
    let modified = fs::metadata(&manifest)
        .expect("manifest metadata")
        .modified()
        .expect("manifest modification time");
    let uri = project.uri("app");
    let symbol = position(source, "answer");
    let mut client = Client::start(&project);
    client.open(&uri, source);
    let initial = client.at("textDocument/hover", &uri, symbol.clone());
    assert!(
        initial["contents"].to_string().contains("fn() -> Int"),
        "initial dependency type: {initial}"
    );

    fs::write(&manifest, configuration("other")).expect("switch dependency path");
    fs::File::options()
        .write(true)
        .open(&manifest)
        .expect("open modified manifest")
        .set_times(fs::FileTimes::new().set_modified(modified))
        .expect("restore manifest modification time");
    assert_eq!(
        fs::metadata(&manifest).unwrap().modified().unwrap(),
        modified
    );

    // No watcher event or source edit: a project request must detect changed
    // dependency contents even when an editor preserves the file timestamp.
    let changed = client.at("textDocument/hover", &uri, symbol.clone());
    assert!(
        changed["contents"].to_string().contains("fn() -> String"),
        "replacement dependency type: {changed}"
    );
    let definition = client.at("textDocument/definition", &uri, symbol);
    assert_eq!(
        definition["uri"],
        file_uri(
            &project
                .directory
                .path()
                .join("packages/other/src/arithmetic.i")
        )
    );
    assert!(client.errors(&uri).is_empty());
    client.finish();
}
