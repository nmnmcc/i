// extension.js with a stand-in for the `vscode` API: it starts `<ilang.path>
// lsp` over stdio for documents of the language `i`, and starts again when
// `ilang.path` changes.

"use strict";

const assert = require("node:assert");
const Module = require("node:module");
const path = require("node:path");
const test = require("node:test");

function fakeVscode(settings) {
  const listeners = [];
  return {
    listeners,
    workspace: {
      workspaceFolders: [{ uri: { fsPath: "/work" } }],
      getConfiguration: () => ({ get: (key) => settings[key] }),
      createFileSystemWatcher: (glob) => ({ glob, dispose() {} }),
      onDidChangeConfiguration: (f) => {
        listeners.push(f);
        return { dispose() {} };
      },
    },
    window: {
      createOutputChannel: (name) => ({ name, dispose() {} }),
      showErrorMessage: () => Promise.resolve(undefined),
    },
    commands: {
      registerCommand: (name, f) => ({ name, f, dispose() {} }),
      executeCommand: () => {},
    },
  };
}

const clients = [];
class FakeClient {
  constructor(id, name, serverOptions, clientOptions) {
    Object.assign(this, { id, name, serverOptions, clientOptions, running: false });
    clients.push(this);
  }
  async start() {
    this.running = true;
  }
  async stop() {
    this.running = false;
  }
}

function loadExtension(vscode) {
  const original = Module._load;
  Module._load = function (request, parent, isMain) {
    if (request === "vscode") {
      return vscode;
    }
    if (request === "vscode-languageclient/node") {
      return { LanguageClient: FakeClient, TransportKind: { stdio: 0 } };
    }
    return original.call(this, request, parent, isMain);
  };
  try {
    const file = path.join(__dirname, "..", "extension.js");
    delete require.cache[require.resolve(file)];
    return require(file);
  } finally {
    Module._load = original;
  }
}

test("starts `i lsp` from the configured path", async () => {
  clients.length = 0;
  const settings = { path: "/opt/i/bin/i" };
  const vscode = fakeVscode(settings);
  const ext = loadExtension(vscode);
  const context = { subscriptions: [] };
  await ext.activate(context);
  assert.strictEqual(clients.length, 1);
  const c = clients[0];
  assert.ok(c.running);
  assert.deepStrictEqual(c.serverOptions.run, { command: "/opt/i/bin/i", args: ["lsp"], transport: 0 });
  assert.deepStrictEqual(c.clientOptions.documentSelector, [
    { scheme: "file", language: "i" },
    { scheme: "untitled", language: "i" },
    { scheme: "file", language: "i-lisp" },
    { scheme: "untitled", language: "i-lisp" },
  ]);

  // A new path starts a new server.
  settings.path = "./target/debug/i";
  vscode.listeners[0]({ affectsConfiguration: (k) => k === "ilang.path" });
  await new Promise((r) => setTimeout(r, 10));
  assert.strictEqual(clients.length, 2);
  assert.ok(!clients[0].running && clients[1].running);
  assert.strictEqual(clients[1].serverOptions.run.command, path.join("/work", "./target/debug/i"));
  await ext.deactivate();
  assert.ok(!clients[1].running);
});

test("uses `i` when no path is set", () => {
  const ext = loadExtension(fakeVscode({ path: "  " }));
  assert.strictEqual(ext.serverPath(), "i");
  const ext2 = loadExtension(fakeVscode({ path: "i-dev" }));
  assert.strictEqual(ext2.serverPath(), "i-dev");
});
