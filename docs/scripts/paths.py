"""Repository locations used by the documentation generators and checks."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# Change this one path when the standard moves to the repository root.
SPEC_PATH = ROOT / 'docs/spec.md'
