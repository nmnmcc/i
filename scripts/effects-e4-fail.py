#!/usr/bin/env python3
"""Port E4 Result-returning std operations to syntax-three Fail needs.

Run from the repository root. --write applies the reviewed conversion and
writes docs/design/effects-e4.md. Without --write, print the candidate list.
Only std/**/*.i and examples/**/*.i are changed. Review the generated output;
manual follow-ups in effects-e4.md retain resource lifetime, existing catches,
and direct named-call pipeline test coverage. Tokens in strings/comments
are left alone. The generated private helpers preserve the algorithms and
module-local Result composition; public functions raise precisely the helper's
error with fail.from_result. This is a one-shot migration, not a formatter.
"""
from pathlib import Path
import argparse
import re

ROOT = Path(__file__).resolve().parents[1]
# These APIs deliberately construct/consume Result data, including callbacks.
KEEP = {('option', 'to_result'), ('list', 'try_map'), ('list', 'try_fold'),
        ('list', 'try_each'), ('cli', 'text'), ('cli', 'integer')}


def masked(s):
    """Keep offsets, newlines and code; blank comments and quoted strings."""
    return re.sub(r'//[^\n]*|"(?:\\.|[^"\\])*"',
                  lambda m: ''.join('\n' if c == '\n' else ' ' for c in m[0]), s)


def close(s, start):
    pairs = {'(': ')', '[': ']', '{': '}', '<': '>'}
    stack = [pairs[s[start]]]
    for i in range(start + 1, len(s)):
        c = s[i]
        if c in '([{':
            stack.append(pairs[c])
        elif c in ')]}':
            if c != stack.pop():
                raise ValueError(f'mismatched delimiter at {i}')
            if not stack:
                return i
    raise ValueError(f'unclosed delimiter at {start}')


def split(s):
    parts, start, depth = [], 0, 0
    for i, c in enumerate(masked(s)):
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
        elif c == ',' and depth == 0:
            if s[start:i].strip():
                parts.append(s[start:i].strip())
            start = i + 1
    if s[start:].strip():
        parts.append(s[start:].strip())
    return parts


def exported(s):
    for m in re.finditer(r'^\(\n(.*?)^\)', s, re.M | re.S):
        if '/' not in m[1] and '=' not in m[1]:
            return set(re.findall(r'^\s*([a-z][a-z_0-9]*)\s*$', m[1], re.M))
    return set()


def definitions():
    found = []
    for p in sorted((ROOT / 'std/src').glob('*.i')):
        mod, s = p.stem, p.read_text()
        if mod == 'result':
            continue
        code, ex = masked(s), exported(s)
        for m in re.finditer(r'^([a-z][a-z_0-9]*) = (?:<([^>]+)>)?\(', code, re.M):
            name = m[1]
            if name not in ex or (mod, name) in KEEP:
                continue
            pend = close(code, m.end() - 1)
            result = re.match(r'\s*->\s*Result\(', code[pend + 1:])
            if not result:
                if not code[pend + 1:].lstrip().startswith('{'):
                    continue
                inferred = {('cli', 'read'): ('A', 'CliError'),
                            ('http', 'body_text'): ('String', 'Nil')}.get((mod, name))
                if inferred is None:
                    continue
                a, e = inferred
                rstart = rend = pend
            else:
                rstart = pend + 1 + result.end() - 1
                rend = close(code, rstart)
                a, e = split(s[rstart + 1:rend])
            params = s[m.end():pend]
            names = [prm.split(':')[0].strip() for prm in split(params)]
            assert all(re.fullmatch(r'[a-z_][a-z_0-9]*', n) for n in names), (p, name)
            tail = code[rend + 1:code.index('{', rend)].strip()
            assert not tail, (p, name, tail)
            tp = f'<{m[2]}>' if m[2] else ''
            old = f'{tp}({params}) -> Result({a}, {e})'
            new = f'{tp}({params}) <- (Fail({e})) -> {a}'
            found.append(dict(path=p, mod=mod, name=name, params=params,
                              names=names, tp=tp, old=old, new=new, a=a, e=e))
    return found


def fail_import(s):
    if re.search(r'\bi/fail\b', masked(s)):
        return s
    m = re.search(r'^\(', masked(s), re.M)
    assert m
    return s[:m.end()] + '\n\ti/fail' + s[m.end():]


def rename_local(s, names):
    code = masked(s)
    # Export lists stay public. All implementation references stay Result-valued.
    groups = list(re.finditer(r'^\(\n(.*?)^\)', code, re.M | re.S))
    end = next(g.end() for g in groups if '/' not in g[1] and '=' not in g[1])
    edits = []
    declarations = list(re.finditer(r'^[a-z_][a-z_0-9]*(?: =|:)', code, re.M))
    shadows = []
    for d in declarations:
        body = code.find('{', d.end())
        if body == -1:
            continue
        finish = close(code, body)
        bound = set(re.findall(r'\b([a-z_][a-z_0-9]*)\s*:', code[d.end():body]))
        shadows.append((body, finish, bound))
    for m in re.finditer(r'(?<![.\w])(' + '|'.join(sorted(names)) + r')\b', code[end:]):
        start, stop = end + m.start(), end + m.end()
        # This corpus has no same-module function-valued uses of converted
        # operations. Rename only top-level definitions and direct calls;
        # names such as calendar.date and cli.help also occur as local data.
        is_definition = (start == 0 or code[start - 1] == '\n') and re.match(r'\s*=', code[stop:])
        if not is_definition and not re.match(r'[ \t]*\(', code[stop:]):
            continue
        if not is_definition and any(a < start < b and m[0] in bound for a, b, bound in shadows):
            continue
        edits.append((start, stop, '_' + m[0] + '_result'))
    for start, stop, value in reversed(edits):
        s = s[:start] + value + s[stop:]
    return s


def bindings(s, funcs):
    """Resolve module aliases and selected unqualified imports."""
    mapping = {}
    code = masked(s)
    for m in re.finditer(r'(?m)^\s*(?:(\w+)\s*=\s*)?(?:\(([^\n]*)\)\s*=\s*)?std/(\w+)\b', code):
        alias, selected, mod = m.groups()
        for f in funcs:
            if f['mod'] != mod:
                continue
            if alias or selected is None:
                mapping[f'{alias or mod}.{f["name"]}'] = f
            if selected and f['name'] in re.findall(r'\b[a-z][a-z_0-9]*\b', selected):
                mapping[f['name']] = f
    return mapping


def catch_call(name, args):
    # Argument evaluation belongs to the caller's Fail. In particular, URI
    # decode_raw can raise BadEscape before bit_array.to_string is entered;
    # catching it here would incorrectly turn it into InvalidUtf8.
    parts = split(args)
    if any('(' in masked(p) or '|>' in masked(p) for p in parts):
        names = [f'e4_call_arg_{i}' for i in range(len(parts))]
        bindings = '\n'.join(f'{n} = {p}' for n, p in zip(names, parts))
        return '{\n' + bindings + f'\nfail.catch(() {{ {name}({", ".join(names)}) }})\n' + '}'
    return f'fail.catch(() {{ {name}({args}) }})'


def rewrite_calls(s, mapping, seen):
    if not mapping:
        return s
    pattern = re.compile(r'(?<![\w.])(' + '|'.join(re.escape(n) for n in sorted(mapping, key=len, reverse=True)) + r')\b')
    def visit(fragment):
        code = masked(fragment)
        out, at = [], 0
        for m in pattern.finditer(code):
            if m.start() < at:
                continue
            name, f = m[0], mapping[m[0]]
            seen.add((f['mod'], f['name']))
            out.append(fragment[at:m.start()])
            rest = m.end()
            while rest < len(code) and code[rest].isspace():
                rest += 1
            pipe = code[:m.start()].rstrip().endswith('|>')
            if rest < len(code) and code[rest] == '(':
                end = close(code, rest)
                args = visit(fragment[rest + 1:end])
                if pipe:
                    args = 'e4_pipe_value' + (', ' + args if args.strip() else '')
                    out.append('(e4_pipe_value) { ' + catch_call(name, args) + ' }')
                else:
                    out.append(catch_call(name, args))
                at = end + 1
            else:
                xs = ', '.join(f'e4_arg_{i}' for i in range(len(f['names'])))
                out.append(f'({xs}) {{ fail.catch(() {{ {name}({xs}) }}) }}')
                at = m.end()
        out.append(fragment[at:])
        return ''.join(out)
    return visit(s)


def compact(s):
    return re.sub(r'\s+', ' ', s).replace(', )', ')').strip()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--write', action='store_true')
    args = ap.parse_args()
    funcs = definitions()
    print(f'{len(funcs)} public std operations')
    if not args.write:
        for f in funcs:
            print(f'{f["mod"]}.{f["name"]}: {compact(f["old"])} => {compact(f["new"])}')
        return
    assert funcs, 'already applied'
    paths = sorted([*(ROOT / 'std').rglob('*.i'), *(ROOT / 'examples').rglob('*.i')])
    callers = {(f['mod'], f['name']): [] for f in funcs}
    sources = {p: p.read_text() for p in paths}
    for p, s in sources.items():
        own = [f for f in funcs if f['path'] == p]
        if own:
            s = rename_local(s, {f['name'] for f in own})
        seen = set()
        s = rewrite_calls(s, bindings(s, funcs), seen)
        for key in seen:
            callers[key].append(str(p.relative_to(ROOT)))
        if own:
            for f in own:
                s += (f'\n/// Reports failure through `Fail({compact(f["e"])})`.\n'
                      f'{f["name"]} = {f["tp"]}({f["params"]}) -> {f["a"]} {{\n'
                      f'\tfail.from_result(_{f["name"]}_result({", ".join(f["names"])}))\n}}\n')
        if own or seen:
            p.write_text(fail_import(s))
    table = '''# E4 standard-library operations

Reviewed against syntax-3 §7.2 row E4 and §2.10.6:

> **Library style.** An operation that may fail reports through `Fail`;
> `Result` is data used after `catch`. About 140 functions of `std/src` and
> `lib/i` that return `Result` today (`json.parse`, `float.divide`, …) change
> in stage E4.

The table lists exported operations with written or inferred `Result` return
types in `std/src`; each now raises its existing error. The private `_name_result`
helpers retain the original implementation and same-module Result composition.
Other std and example callers use `fail.catch` to keep their previous Result
handling, including every success/error assertion. Function-valued uses are
adapted with a function literal; piped calls keep their first argument.
Arguments containing calls are evaluated before entering the adapter catch,
so their errors still reach the caller (notably URI escape validation).
Parameter names in this table document arguments, rather than type labels.
Implicit needs of the original bodies continue to be inferred.

Per-function choices for Result data:

- All of `std/result` and `option.to_result` construct or consume Result data.
- `list.try_map`, `try_fold`, and `try_each` sequence Result-valued callbacks;
  these explicitly choose a Result-based interface, alongside effectful map/fold/each.
- `cli.text` and `cli.integer` are Result-valued conversion callbacks stored in
  `Parser`; keep that protocol. The parsing/help operations themselves raise.
- `schema.Schema.read` and the private parser/codec helpers keep their stored
  Result protocol; public decode/parse operations raise.
- CSV streams of Result rows and stream `from_results`/`to_results` keep Result
  elements as data; parsing a complete document raises.
- `http.body_text` and `cli.read` have inferred Result returns; both are
  included below because they forward a failing operation.

Only `std/**`, examples, this table, and the migration script are owned here.
The `resource.acquire`/`Scope` library change in §2.10.9 requires `lib/i`, which
this lane is explicitly instructed not to edit. Convert its std log.file
caller separately using unchanged resource.ensure to retain per-entry release.
Review pre-existing catches (especially random_bytes_service_error) and piped
calls in tests before committing the generated rewrite.

| Function | Old type | New type | Callers adapted (module-local calls retain private helpers) |
|---|---|---|---|
'''
    for f in funcs:
        key = (f['mod'], f['name'])
        cells = [f'`{f["mod"]}.{f["name"]}`', f'`{compact(f["old"])}`', f'`{compact(f["new"])}`',
                 ', '.join(f'`{p}`' for p in callers[key]) or 'No external std/example callers']
        table += '| ' + ' | '.join(c.replace('|', '\\|') for c in cells) + ' |\n'
    (ROOT / 'docs/design/effects-e4.md').write_text(table)


if __name__ == '__main__':
    main()
