import gmpy2 as g, math, struct, time, json, random
from functools import lru_cache
Q=g.mpq
SIGN=1<<63; MAX=0x7fefffffffffffff
M=Q((1<<53)-1)*Q(2)**971; T=M+Q(2)**970

def bits(f):return struct.unpack('>Q',struct.pack('>d',f))[0]
def fl(b):return struct.unpack('>d',struct.pack('>Q',b))[0]
def rbits(q):
    if not q:return 0
    sign=SIGN if q<0 else 0;q=abs(q)
    if q>=T:return sign|0x7ff0000000000000
    n,d=q.numerator,q.denominator
    e=int(n.bit_length()-d.bit_length())
    if q<Q(2)**e:e-=1
    shift=1074 if e< -1022 else 52-e
    a,b=(n<<shift,d) if shift>=0 else (n,d<<-shift)
    k,r=divmod(a,b);k+=2*r>b or (2*r==b and k%2)
    if e< -1022:return sign|int(k)
    if k==1<<53:k>>=1;e+=1
    return sign|((e+1023)<<52)|int(k-(1<<52))
def add(a,b):return a[0]+b[0],a[1]+b[1]
def neg(a):return -a[1],-a[0]
def sub(a,b):return add(a,neg(b))
def mul(a,b):
    ps=[x*y for x in a for y in b];return min(ps),max(ps)
def div(a,b):
    assert b[0]*b[1]>0
    return mul(a,(1/b[1],1/b[0]))
def point(x):return (Q(x),Q(x))
def scale(a,x):return mul(a,point(x))
metrics={}
def observe(q):
    metrics['max_bits']=max(metrics.get('max_bits',0),int(q.numerator.bit_length()),int(q.denominator.bit_length()))
def smallatan(z,p):
    z=Q(z);s=Q(0);term=z;n=0;eps=Q(2)**-p
    while True:
        s+=term/(2*n+1);observe(s);n+=1;term*=-z*z
        rem=abs(term)/(2*n+1)
        if rem<=eps:break
    metrics['atan_terms']=metrics.get('atan_terms',0)+n
    return s-rem,s+rem
@lru_cache(None)
def pi(p):return sub(scale(smallatan(Q(1,5),p+6),16),scale(smallatan(Q(1,239),p+6),4))
def atan(z,p):
    z=Q(z)
    if z<0:return neg(atan(-z,p))
    if z>1:return sub(scale(pi(p+3),Q(1,2)),atan(1/z,p+3))
    if z>Q(1,2):return add(smallatan(Q(1,2),p+3),smallatan((z-Q(1,2))/(1+z/2),p+3))
    return smallatan(z,p)
def nearest(q):
    v=q+Q(1,2);return int(v.numerator//v.denominator)
def trig(x,p):
    x=Q(x);prec=p+max(0,int(abs(x).numerator.bit_length()-abs(x).denominator.bit_length()))+8
    while True:
        pp=pi(prec);quot=div(point(x),scale(pp,Q(1,2)))
        a,b=map(nearest,quot)
        if a==b:
            r=sub(point(x),scale(pp,Q(a,2)))
            if r[1]-r[0]<=Q(2)**-p and max(map(abs,r))<1:break
        prec*=2
    metrics['pi_precision']=prec
    m=(r[0]+r[1])/2;d=(r[1]-r[0])/2
    vals=[]
    for cos in [False,True]:
        term=Q(1) if cos else m;s=Q(0);n=0
        while True:
            s+=term;observe(s);n+=1
            term*=-m*m/Q((2*n-1)*(2*n) if cos else (2*n)*(2*n+1))
            if abs(term)<=Q(2)**-p:break
        metrics['cos_terms' if cos else 'sin_terms']=n
        vals.append((s-abs(term)-d,s+abs(term)+d))
    sn,cs=vals
    for _ in range(a%4):sn,cs=cs,neg(sn)
    return sn,cs

def sqrtiv(v,p):
    scaled=v*Q(2)**(2*p);k=g.isqrt(scaled.numerator//scaled.denominator);lo=Q(k)*Q(2)**-p
    return lo,lo if lo*lo==v else Q(k+1)*Q(2)**-p

def atan2iv(y,x,p):
    if not y:return point(0) if x>0 else pi(p)
    if not x:return scale(pi(p),Q(1 if y>0 else -1,2))
    if x>0:return atan(y/x,p)
    return sub(pi(p),atan(abs(y/x),p)) if y>0 else add(neg(pi(p)),atan(abs(y/x),p))
def evaluate(name,args):
    x=Q(args[0]);p=128
    if name in ['sin','tan','atan','asin','radians','degrees'] and not x:return bits(args[0]),p
    if name=='cos' and not x:return bits(1.),p
    if name in ['asin','acos'] and abs(x)>1:return None,p
    if name=='acos' and x==1:return 0,p
    if name=='atan2' and not x:
        y=args[0];xx=args[1]
        if not (bits(xx)&SIGN):return bits(y),p
    while True:
        if name in ['sin','cos','tan']:
            sn,cs=trig(x,p)
            if name=='tan' and cs[0]*cs[1]<=0:p*=2;continue
            val=sn if name=='sin' else cs if name=='cos' else div(sn,cs)
        elif name=='atan':val=atan(x,p)
        elif name in ['asin','acos']:
            if abs(x)==1:
                val=scale(pi(p),Q(1 if x>0 else -1,2)) if name=='asin' else pi(p)
            else:
                lo,hi=sqrtiv(1-x*x,p)
                if lo==0:p*=2;continue
                ends=[atan2iv(x,lo,p),atan2iv(x,hi,p)] if name=='asin' else [atan2iv(lo,x,p),atan2iv(hi,x,p)]
                val=min(v[0] for v in ends),max(v[1] for v in ends)
        elif name=='atan2':
            val=atan2iv(x,Q(args[1]),p)
            if not x and bits(args[0])&SIGN:val=neg(val)
        elif name=='radians':val=scale(pi(p),x/180)
        elif name=='degrees':val=div(point(x*180),pi(p))
        elif name=='hypot':
            s=x*x+Q(args[1])**2
            if s>=T*T:return None,p
            if s>=M*M:return MAX,p
            l,h=0,MAX
            while h-l>1:
                mid=(l+h)//2
                if Q(fl(mid))**2<=s:l=mid
                else:h=mid
            mid=(Q(fl(l))+Q(fl(h)))/2
            return l if s<mid*mid or (s==mid*mid and l%2==0) else h,p
        a,b=map(rbits,val)
        if a==b:return (None if a&0x7ff0000000000000==0x7ff0000000000000 else a),p
        p*=2
        assert p<=8192,(name,args,p)

def oracle(name,args):
    if name in ['asin','acos'] and abs(args[0])>1:return None
    vals=[]
    for rnd in [g.RoundDown,g.RoundUp]:
        with g.context(precision=4096,round=rnd):
            aa=[g.mpfr(v) for v in args]
            if name in ['radians','degrees']:
                negative=bool(bits(args[0])&SIGN)
                direction=(g.RoundUp if rnd==g.RoundDown else g.RoundDown) if negative else rnd
                with g.context(precision=4096,round=direction):
                    if name=='radians':
                        magnitude=abs(aa[0])*g.const_pi()/180
                    else:
                        other=g.RoundUp if direction==g.RoundDown else g.RoundDown
                        with g.context(precision=4096,round=other): pp=g.const_pi()
                        magnitude=abs(aa[0])*180/pp
                r=-magnitude if negative else magnitude
            else:r=getattr(g,name)(*aa)
        with g.context(round=g.RoundToNearest): vals.append(bits(float(r)))
    assert vals[0]==vals[1],(name,args,vals)
    b=vals[0];return None if b&0x7ff0000000000000==0x7ff0000000000000 else b

if __name__=='__main__':
    print('MPFR',g.mpfr_version(),flush=True)
    tiny=fl(1);normal=fl(1<<52);maxf=fl(MAX)
    xs=[0.,-0.,tiny,-tiny,normal,math.nextafter(normal,0),1.,-1.,.5,math.nextafter(1.,0),math.nextafter(1.,2),math.pi/2,math.nextafter(math.pi/2,0),math.nextafter(math.pi/2,math.inf),math.pi,1e20,1e300,maxf]
    rng=random.Random(3917);xs += [fl(rng.randrange(MAX)) for _ in range(4)]
    rows=[]
    for name in ['sin','cos','tan','atan','asin','acos','radians','degrees']:
        for x in xs:
            metrics.clear();st=time.perf_counter();b,p=evaluate(name,[x]);sec=time.perf_counter()-st
            expected=oracle(name,[x]);assert b==expected,(name,x,hex(b) if b else b,expected)
            row=dict(fn=name,x=x.hex(),bits=b,p=p,seconds=sec,**metrics);rows.append(row)
            print(json.dumps(row),flush=True)
    for name in ['hypot','atan2']:
        pairs=[(a,b) for a in [0.,-0.,tiny,-tiny,maxf,-maxf] for b in [0.,-0.,tiny,-tiny,maxf,-maxf]]+[(3.,4.),(normal,tiny),(1.,tiny)]
        for args in pairs:
            metrics.clear();st=time.perf_counter();b,p=evaluate(name,args);sec=time.perf_counter()-st
            expected=oracle(name,args);assert b==expected,(name,args,b,expected)
            rows.append(dict(fn=name,args=[x.hex() for x in args],bits=b,p=p,seconds=sec,**metrics))
    json.dump(rows,open('/tmp/review-std3/math_results.json','w'),indent=2)
    print('PASS',len(rows),flush=True)
