// Tokenizes test/sample.i with the grammar, as VS Code does, and compares
// the scopes of every token with test/sample.i.snap. Run with
// `UPDATE=1 npm test` to write the snapshot again.

"use strict";

const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const oniguruma = require("vscode-oniguruma");
const textmate = require("vscode-textmate");

const grammarPath = path.join(__dirname, "..", "syntaxes", "i.tmLanguage.json");
const samplePath = path.join(__dirname, "sample.i");
const snapshotPath = path.join(__dirname, "sample.i.snap");

async function loadGrammar() {
  const wasm = fs.readFileSync(require.resolve("vscode-oniguruma/release/onig.wasm"));
  await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
  const registry = new textmate.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (sources) => new oniguruma.OnigScanner(sources),
      createOnigString: (s) => new oniguruma.OnigString(s),
    }),
    loadGrammar: async (scopeName) => {
      if (scopeName !== "source.i") {
        return null;
      }
      const text = fs.readFileSync(grammarPath, "utf8");
      return textmate.parseRawGrammar(text, grammarPath);
    },
  });
  return registry.loadGrammar("source.i");
}

/** Each token of the text, as `text  scope scope …` without `source.i`. */
function tokenize(grammar, text) {
  const out = [];
  let stack = textmate.INITIAL;
  for (const line of text.split("\n")) {
    const result = grammar.tokenizeLine(line, stack);
    for (const t of result.tokens) {
      const s = line.slice(t.startIndex, t.endIndex);
      if (s.trim() === "") {
        continue;
      }
      const scopes = t.scopes.filter((x) => x !== "source.i");
      out.push(`${JSON.stringify(s)}  ${scopes.join(" ")}`);
    }
    stack = result.ruleStack;
  }
  return out;
}

/** The scopes of each token with exactly this text. */
function scopesOf(tokens, text) {
  const prefix = `${JSON.stringify(text)}  `;
  const lines = tokens.filter((t) => t.startsWith(prefix));
  assert.ok(lines.length > 0, `no token ${text}`);
  return lines.map((l) => l.slice(prefix.length).split(" "));
}

test("the grammar is valid JSON, and its regexes compile", () => {
  const raw = JSON.parse(fs.readFileSync(grammarPath, "utf8"));
  assert.strictEqual(raw.scopeName, "source.i");
  const regexes = [];
  const visit = (v) => {
    if (Array.isArray(v)) {
      v.forEach(visit);
    } else if (v && typeof v === "object") {
      for (const [k, x] of Object.entries(v)) {
        if ((k === "match" || k === "begin" || k === "end") && typeof x === "string") {
          regexes.push(x);
        } else {
          visit(x);
        }
      }
    }
  };
  visit(raw);
  assert.ok(regexes.length > 20);
  return loadGrammar().then(() => {
    for (const r of regexes) {
      assert.doesNotThrow(() => new oniguruma.OnigScanner([r]), `regex ${r}`);
    }
  });
});

test("the package contributes the grammar", () => {
  const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, "..", "package.json"), "utf8"));
  const g = pkg.contributes.grammars.find((x) => x.language === "i");
  assert.strictEqual(g.scopeName, "source.i");
  assert.ok(fs.existsSync(path.join(__dirname, "..", g.path)));
});

test("token scopes of sample.i", async () => {
  const grammar = await loadGrammar();
  const tokens = tokenize(grammar, fs.readFileSync(samplePath, "utf8"));
  // Some token with this text has the scope.
  const has = (text, scope) =>
    assert.ok(
      scopesOf(tokens, text).some((s) => s.includes(scope)),
      `${text}: ${scope} not in ${JSON.stringify(scopesOf(tokens, text))}`,
    );

  has("////", "comment.line.documentation.module.i");
  has("///", "comment.line.documentation.i");
  has("(", "punctuation.definition.header.begin.i");
  has("?", "keyword.operator.guard.i");
  has("|>", "keyword.operator.match.i");
  has("A", "entity.name.type.parameter.i");
  for (const word of ["import", "export", "case", "if"]) {
    assert.ok(scopesOf(tokens, word).every((s) => !s.some((x) => x.startsWith("keyword.control"))));
  }
  has("@", "punctuation.definition.annotation.i");
  for (const a of ["test", "external", "host", "deprecated"]) {
    has(a, "storage.type.annotation.i");
  }
  has("\\t", "constant.character.escape.i");
  has("\\u{1F600}", "constant.character.escape.unicode.i");
  has("\\q", "invalid.illegal.escape.i");
  has("over two lines", "string.quoted.double.i");
  has("0xFF_ff", "constant.numeric.integer.hexadecimal.i");
  has("0o17", "constant.numeric.integer.octal.i");
  has("0b1010_0101", "constant.numeric.integer.binary.i");
  has("1_000", "constant.numeric.integer.decimal.i");
  has("3.5e10", "constant.numeric.float.i");
  has("6.25e-3", "constant.numeric.float.i");
  has("2.", "constant.numeric.float.i");
  has("User", "entity.name.type.i");
  has("Shape", "entity.name.type.i");
  has("Some", "entity.name.type.i");
  has("String", "support.type.builtin.i");
  has("True", "constant.language.i");
  for (const op of ["+.", "-.", "*.", "/."]) {
    has(op, "keyword.operator.arithmetic.float.i");
  }
  for (const op of ["<.", "<=.", ">.", ">=."]) {
    has(op, "keyword.operator.comparison.float.i");
  }
  has("^", "keyword.operator.error.i");
  has("!", "keyword.operator.environment.i");
  has("->", "keyword.operator.arrow.i");
  has("|>", "keyword.operator.pipe.i");
  has("..", "keyword.operator.spread.i");
  has("<<", "punctuation.definition.bit-array.begin.i");
  has(">>", "punctuation.definition.bit-array.end.i");
  for (const f of ["u16be", "utf8", "f64le", "bits"]) {
    has(f, "storage.type.format.i");
  }
  has("i/fail", "entity.name.namespace.i");
  has("std/option", "entity.name.namespace.i");
  has("decode", "entity.name.namespace.alias.i");
  has("__ignored", "variable.language.discard.i");
  has("plus", "entity.name.function.definition.i");
  has("map", "entity.name.function.call.i");
  has("name", "variable.parameter.i");
  // `t.0.1` is two tuple parts, not a float.
  assert.ok(!tokens.some((t) => t.startsWith('"0.1"')), "0.1 in t.0.1");

  const actual = tokens.join("\n") + "\n";
  if (process.env.UPDATE || !fs.existsSync(snapshotPath)) {
    fs.writeFileSync(snapshotPath, actual);
  }
  assert.strictEqual(actual, fs.readFileSync(snapshotPath, "utf8"));
});


test("header scopes stop at the first item", async () => {
  const grammar = await loadGrammar();
  const tokens = tokenize(grammar, "f = () {\n  (a, b) = (1, 2)\n  a\n}\n");
  assert.ok(!tokens.some((t) => t.includes("meta.header.i")));
});
