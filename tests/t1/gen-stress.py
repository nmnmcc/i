#!/usr/bin/env python3
"""Generate the literal-types §9 packages in the accepted syntax."""
import argparse
from pathlib import Path


def source(part, n):
    keys = [f'"k{i:05}"' for i in range(n)]
    if part == 'keys':
        return 'keys = [' + ', '.join(keys) + ']\n'
    if part == 'pick':
        return 'pick = (s: String) { s |> {\n' + ''.join(f'{s} -> {i}\n' for i, s in enumerate(keys)) + '_ -> -1\n} }\n'
    if part == 'calls':
        # Two statements make the RHS indirect (§6.10). The returned
        # function is instantiated once; the local f has a shared monotype.
        return 'calls = () {\nf = { g = (x) { Nil }\ng }\n' + ''.join(f'f({s})\n' for s in keys) + 'Nil\n}\n'
    if part == 'alias':
        return 'Many = ' + ' | '.join(keys) + '\n' + ''.join(f'f{i} = (x: Many) {{ x }}\n' for i in range(100))
    if part == 'nested':
        return 'nested = (p: (String, String)) { p |> {\n' + ''.join(f'({a}, {b}) -> 0\n' for a in keys for b in keys) + '_ -> 1\n} }\n'
    raise ValueError(part)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('n', type=int)
    parser.add_argument('part', choices=['keys', 'pick', 'calls', 'alias', 'nested'])
    args = parser.parse_args()
    args.directory.mkdir(parents=True, exist_ok=True)
    (args.directory / 'src').mkdir(exist_ok=True)
    (args.directory / 'i.toml').write_text('name = "stress"\n')
    (args.directory / 'src/main.i').write_text(source(args.part, args.n))


if __name__ == '__main__':
    main()
