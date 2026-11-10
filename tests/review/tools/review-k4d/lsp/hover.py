import subprocess,json,sys,os,re,time,threading
L='/home/agent/Programming/i/.rd/std1/target/release/i'
def run(proj):
    path=os.path.join(proj,'src/main.i'); text=open(path).read(); uri='file://'+path
    p=subprocess.Popen([L,'lsp'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    err=[]
    threading.Thread(target=lambda:err.append(p.stderr.read()),daemon=True).start()
    def send(m):
        b=json.dumps(m).encode(); p.stdin.write(b'Content-Length: %d\r\n\r\n'%len(b)+b); p.stdin.flush()
    def recv():
        h=b''
        while not h.endswith(b'\r\n\r\n'):
            c=p.stdout.read(1)
            if not c: return None
            h+=c
        n=int(re.search(rb'Content-Length: (\d+)',h).group(1)); return json.loads(p.stdout.read(n))
    def req(i,method,params):
        send({'jsonrpc':'2.0','id':i,'method':method,'params':params})
        while True:
            m=recv()
            if m is None: return None
            if m.get('id')==i and 'method' not in m: return m
    r=req(1,'initialize',{'processId':None,'rootUri':'file://'+proj,'capabilities':{}})
    if r is None: print('init failed',err); return
    send({'jsonrpc':'2.0','method':'initialized','params':{}})
    send({'jsonrpc':'2.0','method':'textDocument/didOpen','params':{'textDocument':{'uri':uri,'languageId':'i','version':1,'text':text}}})
    n=0; shown=set(); i=2
    for ln,line in enumerate(text.split('\n')):
        for m in re.finditer(r'[A-Za-z_][A-Za-z0-9_]*',line):
            i+=1
            r=req(i,'textDocument/hover',{'textDocument':{'uri':uri},'position':{'line':ln,'character':m.start()}})
            if r is None:
                time.sleep(0.3); print('SERVER DIED at',ln+1,m.group(), (err[0][-600:] if err else b'').decode(errors='replace')); return
            n+=1
            res=r.get('result')
            if res: 
                c=res.get('contents'); v=c.get('value') if isinstance(c,dict) else str(c)
                shown.add((m.group(),v.replace('\n',' ')[:110]))
            if 'error' in r: print('ERR',r['error'])
    print(proj.split('/')[-1],'hovers',n,'with result',len(shown))
    if len(sys.argv)>2 and sys.argv[1]=='-v':
        for s in sorted(shown): print('   ',s)
    p.kill()
args=[a for a in sys.argv[1:] if a!='-v']
for a in args: run('/tmp/review-k4d/proj/'+a)
