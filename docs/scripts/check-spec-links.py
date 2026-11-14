#!/usr/bin/env python3
"""Check built page references, prose length and runnable-example counts."""
import argparse
from pathlib import Path
import sys
from spec_links import measure
from paths import ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dist', type=Path, default=Path(__file__).resolve().parents[1] / 'dist')
    parser.add_argument('--content', type=Path, default=ROOT / 'docs/content')
    parser.add_argument('--enforce-area', action='append', default=[],
                        choices=['tour', 'getting-started', 'guides', 'library', 'standard', 'contributing'],
                        help='make prose/example warnings fail for this area; repeatable')
    args = parser.parse_args()
    pages = sorted(args.dist.rglob('*.html'))
    if not pages:
        parser.error('no HTML pages found; build the site first')
    failures = warnings = 0
    for page in pages:
        result = measure(page, args.dist, args.content, args.enforce_area)
        print(f"{result['page']}: {result['words']} prose words; {result['examples']} runnable examples")
        for issue in result['failures']:
            print(f'{page}: error: {issue}', file=sys.stderr)
        for issue in result['warnings']:
            print(f'{page}: warning: {issue}', file=sys.stderr)
        failures += bool(result['failures'])
        warnings += bool(result['warnings'])
    print(f'Per-page checks: {len(pages)} HTML pages; {failures} failures; {warnings} budget warnings.')
    return bool(failures)


if __name__ == '__main__':
    sys.exit(main())
