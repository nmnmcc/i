import os,subprocess,pathlib,sys
p=pathlib.Path('/proc/1457322/environ');e=os.environ.copy()
if p.exists():
 for s in p.read_bytes().split(b'\0'):
  if s.startswith(b'PATH='):e['PATH']=s[5:].decode()
 pathlib.Path('/tmp/review-a1/build-path').write_text(e['PATH'])
else:e['PATH']=pathlib.Path('/tmp/review-a1/build-path').read_text()
d=pathlib.Path('/tmp/review-a1/target/debug/deps');name=sys.argv[1]
cmd=['/nix/store/f10b7wbslr3xhfi97s8accyj3s42gw8q-rust-minimal-1.99.1/bin/rustc','--edition=2024',f'/tmp/review-a1/{name}.rs','-L',f'dependency={d}','-o',f'/tmp/review-a1/{name}']
for n in sys.argv[2:]:cmd+=['--extern',f'{n}={next(d.glob("lib"+n+"-*.rlib"))}']
subprocess.run(cmd,env=e,check=True)
