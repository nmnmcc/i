#!/usr/bin/env python3
"""Diagnostic R2 WAT inventory, not an acceptance-test replacement.

Run from the repository root:
  python3 tests/r2-wat-audit.py REFERENCE DUMP OUTPUT_DIRECTORY

Every shared file gets a byte-equality/error classification. Function diffs
then resolve source owners, data/site operands and constructor numbering.
The second-pass structural substitutions isolate known B frame and R2 closure
representations; they do not establish their correctness. See r2-wat.md for
the independent tests, unsupported stock-normalizer forms and manual residual
review. Never use this inventory to bless a behavioral expectation.
"""
import pathlib,runpy,re,difflib,json,collections,sys
base=pathlib.Path(sys.argv[1]); current=pathlib.Path(sys.argv[2]); out=pathlib.Path(sys.argv[3]);out.mkdir(exist_ok=True)
m=runpy.run_path('scripts/wat-diff')
STRING=m['STRING']; end=m['end_form']; funcs=m['FUNC']; unpack=m['wat_bytes']
def context(text):
 strings={v[1]:unpack(v[2]).decode('utf-8') for v in re.finditer(r'(?m)^\(data (\$s\d+) ('+STRING+r')\)$',text)}
 install=re.search(r'\(call \$constructors_install',text)
 names=[]
 if install:
  form=text[install.start():end(text,install.start())]
  names=[strings[v] for v in re.findall(r'array.new_data \$bytes (\$s\d+)',form)]
 return names

def norms(text,ctor_map,is_new):
 # The stock audit cannot decode the synthetic -1 site marker in some old
 # dumps. Retain that fact in the per-file report rather than bless its bytes.
 site_error=None
 try:
  text=re.sub(r'(\(call \$(?:shadow_site|shadow_leave_tail) \(i32.const )-1(\)\))', r'\1"<none>"\2', text)
  text=m['renumber'](text)
 except Exception as e:
  site_error=str(e)
  # For diagnostic body comparison only, numeric site operands are placeholders.
  # This is explicitly not a successful scripts/wat-diff result.
  text=re.sub(r'(\(call \$(?:shadow_site|shadow_leave_tail) \(i32.const )-?\d+',r'\1<site>',text)
  strings={v[1]:unpack(v[2]).hex() for v in re.finditer(r'(?m)^\(data (\$s\d+) ('+STRING+r')\)$',text)}
  text=re.sub(r'(array.new_data \$bytes )(\$s\d+)',lambda v:v[1]+'$string:'+strings[v[2]],text)
 type_pairs=collections.defaultdict(set)
 for c,t in re.findall(r'(?:\(call \$record|\(struct.new \$obj \(i32.const 5\)) \(i32.const (\d+)\) \(i64.const (\d+)\)',text):
  if int(c) in ctor_map: type_pairs[int(t)].add(ctor_map[int(c)])
 type_map={t:min(c) for t,c in type_pairs.items()}
 def normalize(body):
  body=re.sub(r'\$k(\d+)_(\d+)',lambda v:'$constant:'+str(ctor_map.get(int(v[1]),'new'+v[1])),body)
  body=re.sub(r'(\(call \$record \(i32.const )(\d+)(\) \(i64.const )(\d+)',lambda v:v[1]+'<ctor:'+str(ctor_map.get(int(v[2]),'new'+v[2]))+'>'+v[3]+'<type:'+str(type_map.get(int(v[4]),'unknown'+v[4]))+'>',body)
  body=re.sub(r'(\(struct.new \$obj \(i32.const 5\) \(i32.const )(\d+)(\) \(i64.const )(\d+)',lambda v:v[1]+'<ctor:'+str(ctor_map.get(int(v[2]),'new'+v[2]))+'>'+v[3]+'<type:'+str(type_map.get(int(v[4]),'unknown'+v[4]))+'>',body)
  body=re.sub(r'(\(call \$is_ctor \(local.get \$\w+\) \(i32.const )(\d+)',lambda v:v[1]+'<ctor:'+str(ctor_map.get(int(v[2]),v[2]))+'>',body)
  body=re.sub(r'(\(call \$is_type \(local.get \$\w+\) \(i64.const )(\d+)',lambda v:v[1]+'<type:'+str(type_map.get(int(v[2]),'unknown'+v[2]))+'>',body)
  if is_new:
   body=re.sub(r'(\(array.get \$env \(local.get \$env\) \(i32.const )(\d+)',lambda v:v[1]+str(int(v[2])-5 if int(v[2])>=6 else 1 if int(v[2])==4 else int(v[2])),body)
   body=re.sub(r'(\(call \$(?:env_set|env_with) [^\n]*?\(i32.const )(\d+)',lambda v:v[1]+str(int(v[2])-5 if int(v[2])>=6 else 1 if int(v[2])==4 else int(v[2])),body)
  # B's frame layout/re-entry rewrite is independently inspected, not ordinary
  # user logic. Keep original diffs in the first report; this pass isolates
  # remaining source operations after removing that known representation seam.
  spans=[]
  for hit in re.finditer(r'\(struct.new \$frame(?=[\s)])',body):
   spans.append((hit.start(),end(body,hit.start())))
  for a,b in reversed(spans): body=body[:a]+'<B-capture-frame>'+body[b:]
  if is_new:
   spans=[]
   for hit in re.finditer(r'\(struct.new \$clo\d+ \(i32.const 7\) \(i32.const 1\)',body):
    a=hit.start();b=end(body,a);form=body[a:b]
    form=form.replace('(i32.const 7) (i32.const 1)', '(i32.const 7) (i32.const 0)',1)
    caps=re.search(r'\(array.new_fixed \$values (\d+) ',form)
    if caps:
     ca=caps.start();ce=end(form,ca);inner=form[ca:ce]
     assert inner.endswith(' (ref.null eq))'),inner
     count=int(caps[1])-1
     inner=re.sub(r'(array.new_fixed \$values )\d+',lambda v:v[1]+str(count),inner, count=1)
     inner=inner[:-len(' (ref.null eq))')]+')'
     form=form[:ca]+inner+form[ce:]
    spans.append((a,b,form))
   for a,b,form in reversed(spans):body=body[:a]+form+body[b:]
  return body
 try:
  text=text.replace(';; i/request entry body', ';; i/request.entry_body').replace(';; i/task entry body', ';; i/task.entry_body').replace(';; i/request entry', ';; i/request.entry').replace(';; i/task entry', ';; i/task.entry')
  w=m['Wat'](text)
  groups=collections.defaultdict(list)
  for name,body in w.funcs:
   owner=w.owners.get(name)
   if owner and not re.fullmatch(r'\$[srj]\d+',name): groups[owner].append(normalize(w.rename(body)))
  return {k:'\n'.join(v) for k,v in groups.items()},site_error
 except Exception as e: return {},str(e)

report=[]; residuals=collections.defaultdict(list)
for old in sorted(base.glob('*.wat')):
 new=current/old.name
 if not new.exists(): report.append({'file':old.name,'status':'missing'});continue
 a,b=old.read_text(),new.read_text()
 if a==b: report.append({'file':old.name,'status':'equal'});continue
 if a.startswith('ERROR') or b.startswith('ERROR'):
  report.append({'file':old.name,'status':'error-transition','old':a[:250],'new':b[:250]});continue
 ca,cb=context(a),context(b)
 am={i+16:i+16 for i in range(len(ca))}; bm={}
 for block in difflib.SequenceMatcher(None,ca,cb,autojunk=False).get_matching_blocks():
  for i in range(block.size): bm[block.b+i+16]=block.a+i+16
 # Intrinsic Result/Assert constructors retain their ids.
 am.update({i:i for i in range(16)});bm.update({i:i for i in range(16)})
 ga,ea=norms(a,am,False);gb,eb=norms(b,bm,True)
 changes=[owner for owner in sorted(ga) if ga[owner]!=gb.get(owner)]
 for owner in changes:
  residuals[owner].append(old.name)
  path=out/(old.name+'.'+owner.replace('/','_').replace(':','_')+'.diff')
  path.write_text('\n'.join(difflib.unified_diff(ga[owner].splitlines(),gb.get(owner,'<removed>').splitlines(),fromfile='baseline',tofile='R2'))+'\n')
 report.append({'file':old.name,'status':'changed','constructor_names':[len(ca),len(cb)],'normalizer_notes':[ea,eb],'changed_owners':changes,'added_owners':sorted(set(gb)-set(ga))})
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
(out/'owners.json').write_text(json.dumps(residuals,indent=2)+'\n')
print('Diagnostic substitutions only; see tests/r2-wat.md')
print(collections.Counter(r['status'] for r in report))
print('changed owner count',len(residuals))
for k,v in residuals.items(): print(k,len(v),v[0])
