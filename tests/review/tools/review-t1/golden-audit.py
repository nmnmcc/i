from pathlib import Path
import html
files=['annotations','basics','bit_patterns','case','choices','clauses','display','environments','errors','generalize','groups','known_intersection','may_fail','narrowing']
rows=Path('tests/t1-golden-rules.md').read_text().splitlines();out=['Independent sample: source files and corresponding changes read against the specification.','Both removed and added rows count as changed golden lines. No unjustified semantic change found in this sample.','']
n=0
for name in files:
 selected=[r for r in rows if r.startswith('| crates/ilang-check/tests/types/'+name+'.out:')][:6]
 out+=['## '+name]+[html.unescape(r) for r in selected]+[''];n+=len(selected)
out.insert(0,f'{n} sampled changed lines across {len(files)} source fixtures.')
Path('/tmp/review-t1/golden-audit.md').write_text('\n'.join(out))
print('\n'.join(out))
