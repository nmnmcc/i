from pathlib import Path
import subprocess,json
R=Path('/home/agent/Programming/i/.rd/a');T=Path('/tmp/review-t1');out=[]
for name in ['tests/spec/11/equality.i','tests/spec/11/stop_tail_service.i','tests/spec/6/blocks.i','tests/spec/6/strings.i']:
 p=T/('original-'+Path(name).stem);(p/'test').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name="spec"\n');(p/'test/main_test.i').write_bytes(subprocess.check_output(['git','show','refs/heads/redesign:'+name],cwd=R))
 for bin in [R/'target/t1/continued/i-baseline-94789e6',R/'target/release/i']:
  r=subprocess.run([str(bin),'check','--offline','--project',str(p)],capture_output=True,text=True)
  out.append(dict(source=name,binary=str(bin),exit=r.returncode,stderr=r.stderr));print(name,bin.name,r.returncode,r.stderr,flush=True)
(T/'original-fixtures.json').write_text(json.dumps(out,indent=2))
