import sys,json,time
sys.path.insert(0,'/tmp/review-std3')
from math_review import *
rows=[]
for y in [fl(1),-fl(1)]:
 for x in [2.,math.nextafter(2.,0),math.nextafter(2.,math.inf),fl(MAX)]:
  metrics.clear();st=time.perf_counter();b,p=evaluate('atan2',[y,x]);o=oracle('atan2',[y,x]);assert b==o,(y,x,b,o)
  rows.append(dict(fn='atan2',args=[y.hex(),x.hex()],bits=b,p=p,seconds=time.perf_counter()-st,**metrics))
for x in [fl(28),fl(29),-fl(28),-fl(29)]:
 b,p=evaluate('radians',[x]);o=oracle('radians',[x]);assert b==o
 rows.append(dict(fn='radians',args=[x.hex()],bits=b,p=p))
json.dump(rows,open('/tmp/review-std3/underflow_results.json','w'),indent=2)
print(json.dumps(rows,indent=2));print('PASS',len(rows))
