"""Replay preserved independent models without their historical absolute paths."""
import os
import json
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

from corpus import CORPUS, sha


def run_models(mpfr_python=None, timeout=120, selected_filter=''):
    work = CORPUS / '.work'
    work.mkdir(exist_ok=True)
    results = []
    with tempfile.TemporaryDirectory(prefix='models-', dir=work) as temp:
        root = Path(temp)
        selected = [(p, sys.executable) for p in sorted((CORPUS / 'tools/record-models').glob('*.py'))]
        selected.append((CORPUS / 'tools/review-std1/uri_model.py', sys.executable))
        math_dir = root / 'math'
        math_dir.mkdir()
        if mpfr_python:
            for name in ['math_review.py', 'extra_math.py', 'hypot_ties.py', 'underflow_review.py']:
                original = CORPUS / 'tools/review-std3' / name
                # Original review paths identify output/import locations, not algorithms.
                target = math_dir / name
                target.write_text(original.read_text().replace('/tmp/review-std3', str(math_dir)))
                selected.append((original, str(mpfr_python)))
        choices = [item for item in selected if not selected_filter or selected_filter in str(item[0])]
        if not choices:
            raise ValueError('no models selected')
        for source, python in choices:
            executed = math_dir / source.name if source.parent.name == 'review-std3' else source
            env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
            with subprocess.Popen([python, str(executed)], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True) as child:
                try:
                    stdout, stderr = child.communicate(timeout=timeout)
                    code = child.returncode
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    stdout, stderr = child.communicate()
                    code = 'timeout'
            result = {'model': str(source.relative_to(CORPUS)), 'source_sha256': sha(source.read_bytes()),
                      'python': python, 'exit': code, 'stdout': stdout.decode(errors='replace'), 'stderr': stderr.decode(errors='replace'),
                      'classification': 'independent model; not compiler implementation validation'}
            if source.parent.name == 'review-std3':
                result['outputs'] = {p.name: json.loads(p.read_text()) for p in sorted(math_dir.glob('*.json'))}
            if source.name == 't3-forgetting-conflict.py':
                result['expected_exit'] = 1
                result['requirement'] = 'Historical witness: its main asserts substitution stability under the old common-label join and must fail with the documented joint §6 conflict.'
                result['matched'] = code == 1 and 'joint §6 requires substitution stability' in result['stderr']
            else:
                result['expected_exit'] = 0
                result['matched'] = code == 0
            results.append(result)
            print(f'model {source.name}: {code}', flush=True)
    return results
