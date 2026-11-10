#!/usr/bin/env python3
"""Compare every repository package and standalone checker/spec fixture.

Use two explicit binaries. The report distinguishes new fixtures from lost
previously accepted programs; a changed diagnostic on an already invalid
program is not a lost program. Runtime checks remain the spec/dev gates.
"""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parents[2]


def run(binary, project):
    p=subprocess.run([binary,'check','--offline','--project',str(project),'--types'],cwd=ROOT,text=True,capture_output=True)
    return {'exit':p.returncode,'diagnostics':p.stderr,'types':p.stdout,'codes':re.findall(r'(?:error|warning)\[([^]]+)\]',p.stderr)}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before');parser.add_argument('after');parser.add_argument('--base-ref',default='b2cfa94');parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    before=str(Path(args.before).resolve());after=str(Path(args.after).resolve())
    files=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],cwd=ROOT,text=True).splitlines()
    projects=sorted({Path(p).parent for p in files if p.endswith('/i.toml') or p=='i.toml'})
    cases=[(str(p),p,None) for p in projects]
    for name in files:
        p=Path(name)
        if p.suffix!='.i' or not (name.startswith('tests/spec/') or name.startswith('crates/ilang-check/tests/types/')):continue
        if any(parent==p.parent or parent in p.parents for parent in projects):continue
        cases.append((name,None,p))
    output=[]
    for name,project,file in cases:
        prior=subprocess.run(['git','cat-file','-e',f'{args.base_ref}:{name if file else str(project/"i.toml")}'],cwd=ROOT,capture_output=True).returncode==0
        if project is not None:
            b=run(before,ROOT/project);a=run(after,ROOT/project)
        else:
            with tempfile.TemporaryDirectory(prefix='ilang-t1-corpus-') as tmp:
                dest=Path(tmp)
                spec=str(file).startswith('tests/spec/')
                (dest/'i.toml').write_text('name = "spec"\n' if spec else 'name = "app"\n')
                folder=dest/('test' if spec else 'src');folder.mkdir()
                (folder/file.name if spec else folder/'main.i').write_text((ROOT/file).read_text())
                b=run(before,dest);a=run(after,dest)
        status='added' if not prior else ('lost' if b['exit']==0 and a['exit']!=0 else 'gained' if b['exit']!=0 and a['exit']==0 else 'kept')
        output.append({'case':name,'status':status,'before':b,'after':a})
    summary={kind:sum(row['status']==kind for row in output) for kind in ('added','kept','gained','lost')}
    args.output.write_text(json.dumps({'summary':summary,'cases':output},indent=2)+'\n')
    print(json.dumps(summary))
    for row in output:
        if row['status']=='lost':print(row['case'],row['after']['diagnostics'])

if __name__=='__main__':main()
