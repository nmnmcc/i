// VS Code client for the i programming language.
//
// Starts `<ilang.path> lsp` as a stdio language server for documents with the
// language id `i`. Plain JavaScript on purpose: there is no build step.

"use strict";

const path = require("path");
const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

/** @type {LanguageClient | undefined} */
let client;
/** @type {vscode.OutputChannel | undefined} */
let outputChannel;

/**
 * The `i` executable: `ilang.path`, or `i` on the `PATH`. A relative path
 * with a directory in it, such as `./target/debug/i`, is taken from the
 * first workspace folder.
 */
function serverPath() {
  const configured = vscode.workspace.getConfiguration("ilang").get("path");
  const value = typeof configured === "string" ? configured.trim() : "";
  if (value.length === 0) {
    return "i";
  }
  const folders = vscode.workspace.workspaceFolders;
  if (!path.isAbsolute(value) && /[\\/]/.test(value) && folders && folders.length > 0) {
    return path.join(folders[0].uri.fsPath, value);
  }
  return value;
}

function createClient() {
  const command = serverPath();
  /** @type {import("vscode-languageclient/node").ServerOptions} */
  const serverOptions = {
    run: { command, args: ["lsp"], transport: TransportKind.stdio },
    debug: { command, args: ["lsp"], transport: TransportKind.stdio },
  };

  /** @type {import("vscode-languageclient/node").LanguageClientOptions} */
  const clientOptions = {
    documentSelector: [
      { scheme: "file", language: "i" },
      { scheme: "untitled", language: "i" },
    ],
    synchronize: {
      fileEvents: [
        vscode.workspace.createFileSystemWatcher("**/i.toml"),
        vscode.workspace.createFileSystemWatcher("**/*.i"),
      ],
    },
    outputChannel,
  };

  return new LanguageClient("ilang", "i Language Server", serverOptions, clientOptions);
}

async function startClient() {
  client = createClient();
  try {
    await client.start();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    vscode.window
      .showErrorMessage(
        `Failed to start the i language server using "${serverPath()} lsp": ${message}. ` +
          "Install the i compiler or set `ilang.path`.",
        "Open Settings",
      )
      .then((choice) => {
        if (choice === "Open Settings") {
          vscode.commands.executeCommand("workbench.action.openSettings", "ilang.path");
        }
      });
  }
}

async function stopClient() {
  if (!client) {
    return;
  }
  const current = client;
  client = undefined;
  try {
    await current.stop();
  } catch {
    // The server may never have started; nothing to clean up.
  }
}

async function restartClient() {
  await stopClient();
  await startClient();
}

/** @param {vscode.ExtensionContext} context */
async function activate(context) {
  outputChannel = vscode.window.createOutputChannel("i Language Server");
  context.subscriptions.push(outputChannel);

  context.subscriptions.push(
    vscode.commands.registerCommand("ilang.restartServer", restartClient),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("ilang.path")) {
        restartClient();
      }
    }),
  );

  await startClient();
}

function deactivate() {
  return stopClient();
}

module.exports = { activate, deactivate, serverPath };
