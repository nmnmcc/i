#!/usr/bin/env python3
"""Limit specification references in the built reader pages."""
import argparse
from pathlib import Path
import sys
from spec_links import measure


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dist', type=Path, default=Path(__file__).resolve().parents[1] / 'dist')
    args = parser.parse_args()
    pages = sorted(args.dist.rglob('*.html'))
    if not pages:
        parser.error('no HTML pages found; build the site first')
    failures = 0
    for page in pages:
        result = measure(page, args.dist)
        if result['failed']:
            expectation = 'no specification links' if result['area'] == 'library' else f"at most {result['limit']} specification references"
            print(f"{page}: expected {expectation}; found {result['sections']} section marks and {result['links']} links", file=sys.stderr)
            failures += 1
    print(f'Specification references checked in {len(pages)} HTML pages; {failures} over limit.')
    return bool(failures)


if __name__ == '__main__':
    sys.exit(main())
