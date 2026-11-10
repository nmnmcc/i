import subprocess,time,sys
L='/home/agent/Programming/i/.rd/std1/target/release/i'; B='/tmp/review-k4d/base-target/release/i'
for p in sys.argv[2:]:
    for name,b in (('lane',L),('base',B)):
        if name not in sys.argv[1]: continue
        t=time.time()
        try:
            r=subprocess.run([b,'check','--project',p,'--types'],capture_output=True,text=True,timeout=100); rc=r.returncode; o=r.stdout+r.stderr
        except subprocess.TimeoutExpired: rc='TIMEOUT'; o=''
        dt=time.time()-t
        errs=sum('error[' in l for l in o.splitlines()); ty=[l for l in o.splitlines() if ' : ' in l]
        print(f'{p:14s} {name} rc={rc} {dt:6.2f}s errors={errs} types={len(ty)} last={ty[-1][:70] if ty else ""}',flush=True)
