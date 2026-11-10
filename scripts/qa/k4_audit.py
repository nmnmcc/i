"""Account for every corpus transition and benchmark difference during K4 integration."""
import collections
import json
import os
from pathlib import Path
import re
import subprocess

from corpus import ROOT, CORPUS, dump, sha

BASELINE = 'd655684bdb809c5a38bf86b61fc5ef26f5bc5392'
STD2 = {
    'annotated', 'client_nil_error', 'client_no_marker', 'error', 'full',
    'minimal_client', 'minimal_client_final', 'pure_catch', 'pure_wrap',
    'replacement', 'unsafe',
}


def saved(path):
    return json.loads(subprocess.check_output(['git', 'show', BASELINE + ':' + path], cwd=ROOT))


def codes(outcome):
    return sorted(set(re.findall(r'error\[([^]]+)\]', outcome['check']['stderr'])))


def audit():
    if os.environ.get('UPDATE') != '1':
        raise SystemExit('audit-k4 requires UPDATE=1')
    before_cases = saved('tests/review/manifest.json')['cases']
    before_rules = saved('tests/review/requirements.json')['requirements']
    rules = json.loads((CORPUS / 'requirements.json').read_text())['requirements']
    cases = json.loads((CORPUS / 'manifest.json').read_text())['cases']
    changes, promotions, remaining, rejected, outside = [], [], [], [], []
    metadata = collections.Counter()
    for case in before_cases:
        identifier = case['id']
        old = saved(case['project'] + '/expected.json')
        new = json.loads((ROOT / case['project'] / 'expected.json').read_text())
        active = [r for r in rules if re.search(r['pattern'], identifier)]
        old_known = [r for r in before_rules if r.get('known_failure') and re.search(r['pattern'], identifier)]
        if old_known and identifier.startswith('review-k4'):
            if any(r.get('known_failure') for r in active):
                remaining.append(identifier)
            else:
                assert active, identifier
                promotions.append({'case': identifier, 'finding': old_known[0]['finding'],
                                   'fixed_by': next((r.get('fixed_by') for r in active if r.get('fixed_by')), None),
                                   'source': old_known[0]['source']})
        for key in ['compiler', 'binary_sha256', 'bound_seconds']:
            if old.get(key) != new.get(key):
                metadata[key] += 1
        a, b = old['outcome'], new['outcome']
        if a == b:
            continue
        entry = {'case': identifier, 'before': {k: v['exit'] for k, v in a.items()},
                 'after': {k: v['exit'] for k, v in b.items()},
                 'before_codes': codes(a), 'after_codes': codes(b),
                 'types_changed': a['check']['stdout'] != b['check']['stdout'],
                 'diagnostics_changed': a['check']['stderr'] != b['check']['stderr'],
                 'before_sha256': sha(json.dumps(a, sort_keys=True).encode()),
                 'after_sha256': sha(json.dumps(b, sort_keys=True).encode())}
        if identifier.startswith('review-k4'):
            entry['reason'] = 'K4 finite row inference, preserved service equations, canonical row display, or explicit entry-run coverage; exact requirements remain separate'
            if a['check']['exit'] == 0 and b['check']['exit'] == 1:
                assert any(r['outcome'] == 'reject' and not r.get('known_failure') for r in active), identifier
                entry['reason'] = ('§4.5 rule 4: incompatible forms of one service constructor' if 'E0515' in codes(b)
                                   else '§§4.8/9.2/10.2: an entry retains an unprovided service')
                rejected.append(entry)
        elif identifier == 'review-js4/settings':
            assert a['check']['exit'] == b['check']['exit'] == 0
            left = a['check']['stdout'].replace('! Await | Owner | Fs', '! Await | Fs | Owner')
            assert left == b['check']['stdout']
            assert all(a[k] == b[k] for k in a if k != 'check')
            entry['reason'] = '§4.4: only the order of Await, Fs and Owner changes; all runtime phases are identical'
            outside.append(entry)
        elif identifier.startswith('review-std2/') and identifier.split('/')[-1] in STD2:
            if identifier.endswith('/unsafe'):
                assert a['check']['exit'] == b['check']['exit'] == 1
                assert codes(a) == ['E0501'] and codes(b) == ['E0515']
                entry['reason'] = 'F2: Sink carries a row; Consumer(Int) and Consumer(Never) must be equal, so E0515 replaces E0501; rejection is preserved'
            elif identifier.endswith('/replacement'):
                assert a['check']['exit'] == b['check']['exit'] == 1
                assert a['check']['stdout'] == b['check']['stdout'] and codes(a) == codes(b) == ['E0515']
                entry['reason'] = 'F2: the same E0515 now names the complete S(Int)/S(String) forms; shown types and rejection are preserved'
            else:
                assert b['check']['exit'] == 0 and all(v['exit'] == 0 for v in b.values())
                assert a['check']['exit'] == 0 or codes(a) == ['E0505']
                entry['reason'] = 'Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels'
            outside.append(entry)
        elif identifier.startswith('review-t1/stress-keys-'):
            assert a['check'] == b['check']
            for phase in a:
                if a[phase]['exit'] != 'timeout' and b[phase]['exit'] != 'timeout':
                    assert a[phase] == b[phase], (identifier, phase)
            entry['reason'] = 'Only censored runtime phases complete under the longer observation budget; checker types/diagnostics and completed outputs are identical; no K4 speedup claimed'
            outside.append(entry)
        else:
            raise AssertionError('unexplained non-K4 difference: ' + identifier)
        changes.append(entry)
    assert not remaining, remaining
    benches = [json.loads(line) for line in (CORPUS / 'bench-k4-comparison.jsonl').read_text().splitlines()]
    bundles = [json.loads((CORPUS / name).read_text()) for name in ['bench-baseline.json', 'bench-k4.json']]
    output_changes = []
    for workload in bundles[0]['catalogue']:
        name = workload['name']
        hashes = [{key: sorted({s[key] for s in bundle['samples'] if s['workload'] == name})
                   for key in ['stdout_sha256', 'stderr_sha256']} for bundle in bundles]
        if hashes[0] != hashes[1]:
            output_changes.append({'workload': name, 'before': hashes[0], 'after': hashes[1]})
    generation_before = saved('tests/review/generation-long.json')
    generation_after = json.loads((CORPUS / 'generation-k4-long.json').read_text())
    generation_changes = {key: {'before': generation_before.get(key), 'after': generation_after.get(key)}
                          for key in ['mode', 'seed', 'count', 'counts', 'failures', 'report_only', 'rules', 'models']
                          if generation_before.get(key) != generation_after.get(key)}
    result = {'baseline': BASELINE, 'baseline_projects': len(before_cases), 'projects': len(cases),
              'metadata_changes': dict(metadata), 'promotions': promotions, 'remaining_k4_known_failures': remaining,
              'accepted_then_rejected': rejected, 'outside_k4_changes': outside, 'differences': changes,
              'benchmark_differences': benches,
              'benchmark_output_differences': output_changes,
              'generation_result_differences': generation_changes,
              'method': 'Compare complete saved observations; semantic requirements and their cited source sentences are independent. Runtime-budget completion is censored evidence, not a numeric performance claim.'}
    dump(CORPUS / 'k4-integration.json', result)
    lines = ['# K4 corpus integration', '', f'Baseline: {BASELINE}. {len(promotions)} K4 known-failure entries promoted. No K4 known failure remains.', '',
             '## Flipped entries', '']
    for finding in sorted({p['finding'] for p in promotions}):
        lines += ['### ' + finding, '']
        lines += ['- ' + p['case'] for p in promotions if p['finding'] == finding]
        lines += ['']
    lines += ['## Baseline-accepted programs now rejected', '', '| Program | Diagnostic | Reason |', '| --- | --- | --- |']
    for row in rejected:
        lines.append('| ' + row['case'] + ' | ' + ', '.join(row['after_codes']) + ' | ' + row['reason'] + ' |')
    lines += ['', '## Every changed entry outside K4', '', '| Program | Reason |', '| --- | --- |']
    lines += ['| ' + r['case'] + ' | ' + r['reason'] + ' |' for r in outside]
    lines += ['', '## Every benchmark workload', '', 'Wall/CPU are seconds; RSS is bytes. A censored baseline has no completed-cost ratio.', '',
              '| Workload | Wall before → after | CPU before → after | RSS before → after | Censored before/after |', '| --- | ---: | ---: | ---: | --- |']
    for row in benches:
        cells = [row['workload']]
        for key in ['wall_seconds', 'cpu_seconds', 'peak_rss_bytes']:
            m = row[key]
            cells.append(f'{m["baseline"]:.6f} → {m["candidate"]:.6f}' if key != 'peak_rss_bytes' else f'{m["baseline"]:.0f} → {m["candidate"]:.0f}')
        cells.append(str(row['censored']))
        lines.append('| ' + ' | '.join(cells) + ' |')
    lines += ['', 'Benchmark output hashes differ for: ' + ', '.join(r['workload'] for r in output_changes) + '.',
              'Exact hashes and all timing/counter differences are in k4-integration.json.', '',
              'Long-generation result differences from the stored long run: ' + json.dumps(generation_changes, ensure_ascii=False) + '.', '']
    (CORPUS / 'k4-integration.md').write_text('\n'.join(lines) + '\n')
    print(json.dumps({'promotions': len(promotions), 'remaining_k4': remaining,
                      'accepted_then_rejected': len(rejected), 'outside_changes': len(outside),
                      'all_changed_observations': len(changes), 'benchmark_workloads': len(benches)}))
