from probe import case,ROOT,BIN
import itertools,subprocess,json
items=['v: Int = 1','Pair(A,B) = (A,B)','parts: (Int,String) = (1,"s")','Pair(Left,Right) = parts','result: (v,Left,Right) = (2,3,"ok")']
results=[]
for i,order in enumerate(itertools.permutations(items)):
 p=ROOT/'order'/str(i);(p/'src').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name="review"\n');(p/'src/main.i').write_text('\n'.join(order)+'\n')
 r=subprocess.run([BIN,'check','--types','--project',str(p)],capture_output=True,text=True,timeout=5)
 (p/'check.txt').write_text(r.stdout+r.stderr);results.append((r.returncode,sorted(r.stdout.splitlines()),r.stderr))
(ROOT/'order/results.json').write_text(json.dumps(results,indent=2));print('permutations',len(results),'unique outcomes',len(set(map(str,results))))
