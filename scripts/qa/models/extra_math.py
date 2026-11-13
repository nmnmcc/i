import sys,json
sys.path.insert(0,'/tmp/review-std3')
from math_review import *
rows=[]
# Round-to-nearest values of large integer multiples of true pi/2, then adjacent floats.
with g.context(precision=4096):
    centers=[float(g.const_pi()*g.mpfr(k)/2) for k in [2,3,127,2**32+1,2**100,2**900]]
xs=[y for x in centers for y in [math.nextafter(x,0),x,math.nextafter(x,math.inf)]]
for x in xs:
    for name in ['sin','cos','tan']:
        b,p=evaluate(name,[x]);o=oracle(name,[x]);assert b==o,(name,x,b,o)
        rows.append(dict(fn=name,x=x.hex(),bits=b,p=p))
# Search independently for values whose real sine lies unusually near a rounding midpoint.
rng=random.Random(8041);best=[]
with g.context(precision=256):
    for i in range(12000):
        x=fl(rng.randrange(1,MAX));y=g.sin(g.mpfr(x));f=float(y)
        neighbor=math.nextafter(f,math.inf if y>g.mpfr(f) else -math.inf)
        mid=(g.mpfr(f)+g.mpfr(neighbor))/2
        dist=abs(y-mid)/abs(g.mpfr(f)-g.mpfr(neighbor))
        best.append((float(dist),x))
for dist,x in sorted(best)[:5]:
    b,p=evaluate('sin',[x]);o=oracle('sin',[x]);assert b==o
    rows.append(dict(fn='sin midpoint search',x=x.hex(),bits=b,p=p,distance_ulp=dist))
json.dump(rows,open('/tmp/review-std3/extra_math_results.json','w'),indent=2)
print('PASS',len(rows));print(json.dumps(rows[-5:],indent=2))
