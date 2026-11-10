from math_review import *
for m,n,factor in [(2**26+1,2**26,1),(2**25+1,2**25,7)]:
    a,b,c=factor*2*m*n,factor*(m*m-n*n),factor*(m*m+n*n)
    for exponent in [0,-1074,970]:
        args=[float(Q(a)*Q(2)**exponent),float(Q(b)*Q(2)**exponent)]
        result,p=evaluate('hypot',args)
        assert result==oracle('hypot',args)
        print([v.hex() for v in args],c,'scale',exponent,'result',hex(result))
