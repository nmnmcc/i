from pathlib import Path
import subprocess
from probe import ROOT,BIN
lib=ROOT/'pkg_lib';(lib/'src').mkdir(parents=True,exist_ok=True);(lib/'i.toml').write_text('name="schema"\n')
(lib/'src/api.i').write_text('''(Schema,make)
Schema(D,E) {Schema(decode:(String)->D,encode:(E)->String)}
make(D,E) = (d:(String)->D,e:(E)->String) {Schema(d,e)}
''')
app=ROOT/'pkg_app';(app/'src').mkdir(parents=True,exist_ok=True)
(app/'i.toml').write_text('name="app"\n[dependencies]\nschema={path="../pkg_lib"}\n')
(app/'src/model.i').write_text('''((Schema,make)=schema/api)
(User)
user=make((s:String)->(String,Int){(s,1)},(u:(String,Int))->String{u.0})
Schema(User,_) = user
''')
client=ROOT/'pkg_client';(client/'src').mkdir(parents=True,exist_ok=True)
(client/'i.toml').write_text('name="client"\n[dependencies]\napp={path="../pkg_app"}\n')
(client/'src/main.i').write_text('''((User)=app/model)
x: User = ("yes",1)
''')
r=subprocess.run([BIN,'check','--project',str(client),'--types'],capture_output=True,text=True);(client/'check.txt').write_text(r.stdout+r.stderr);print(r.returncode,r.stdout,r.stderr)
