import subprocess,json,threading,queue
from pathlib import Path
from probe import case,BIN,ROOT
class LSP:
 def __init__(self,p):
  self.p=subprocess.Popen([BIN,'lsp'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=open(p/'lsp.stderr','w'))
  self.q=queue.Queue(); self.id=0; self.log=[]
  def reader():
   while True:
    line=self.p.stdout.readline()
    if not line:break
    if line.lower().startswith(b'content-length:'):
     size=int(line.split(b':')[1]);self.p.stdout.readline();self.q.put(json.loads(self.p.stdout.read(size)))
  threading.Thread(target=reader,daemon=True).start()
  self.req('initialize',{'capabilities':{},'rootUri':p.as_uri()}); self.send('initialized',{})
 def send(self,m,params,id=None):
  x={'jsonrpc':'2.0','method':m,'params':params}
  if id is not None:x['id']=id
  b=json.dumps(x).encode();self.p.stdin.write(('Content-Length: %d\r\n\r\n'%len(b)).encode()+b);self.p.stdin.flush()
 def req(self,m,params):
  self.id+=1;self.send(m,params,self.id)
  while True:
   x=self.q.get(timeout=15);self.log.append(x)
   if x.get('id')==self.id:return x
 def close(self):
  self.req('shutdown',None);self.send('exit',None);self.p.wait(timeout=5)
def probe(name):
 p=ROOT/name;s=(p/'src/main.i').read_text();uri=(p/'src/main.i').as_uri();l=LSP(p)
 l.send('textDocument/didOpen',{'textDocument':{'uri':uri,'languageId':'i','version':1,'text':s}})
 results={}
 for label,needle,delta in [('binder','Item',0),('nested_head','Pair(Box',5),('source','= v',2),('use','x: Item',3)]:
  pos=s.find(needle)
  if pos<0:continue
  pos+=delta;line=s.count('\n',0,pos);ch=pos-(s.rfind('\n',0,pos)+1)
  for method in ['hover','definition']:
   results[label+'_'+method]=l.req('textDocument/'+method,{'textDocument':{'uri':uri},'position':{'line':line,'character':ch}})
 results['symbols']=l.req('textDocument/documentSymbol',{'textDocument':{'uri':uri}})
 l.close();(p/'lsp.json').write_text(json.dumps({'results':results,'messages':l.log},indent=2)); print(name,json.dumps(results))
if __name__=='__main__':
 probe('nested');probe('recovery');probe('cycle_alias')
