from pathlib import Path
from datetime import date
import random
root=Path('/tmp/review-std2/arithmetic'); (root/'test').mkdir(parents=True,exist_ok=True);(root/'i.toml').write_text('name = "arithmetic"\n')
def leap(y): return y%4==0 and (y%100!=0 or y%400==0)
def jan(y): return 365*y+(y+3)//4-(y+99)//100+(y+399)//400
lengths=[31,28,31,30,31,30,31,31,30,31,30,31]
def oracle(y,m,d): return jan(y)-jan(1970)+sum(lengths[:m-1])+(m>2 and leap(y))+d-1
def forward(y,m,d):
 y1=y-(m<=2);era=y1//400;yoe=y1-era*400;mp=m-3 if m>2 else m+9;doy=(153*mp+2)//5+d-1
 return era*146097+365*yoe+yoe//4-yoe//100+doy-719468
def backward(day):
 z=day+719468;era=z//146097;doe=z-era*146097;yoe=(doe-doe//1460+doe//36524-doe//146096)//365;y=yoe+era*400;doy=doe-(365*yoe+yoe//4-yoe//100);mp=(5*doy+2)//153;d=doy-(153*mp+2)//5+1;m=mp+3 if mp<10 else mp-9
 return y+(m<=2),m,d
rows=[]
for y in [-10**100,-401,-400,-100,-4,-1,0,1,1582,1900,1970,2000,9999,10000,10**100]:
 for m,d in [(1,1),(2,28),(3,1),(12,31)]+([(2,29)] if leap(y) else []): rows.append((y,m,d))
for y in range(-800,801):
 for m in range(1,13):
  for d in [1,lengths[m-1]+(m==2 and leap(y))]:
   n=oracle(y,m,d); assert n==forward(y,m,d); assert backward(n)==(y,m,d)
for n in range(-1000000,-1000000+146098): assert oracle(*backward(n))==n
for n in [-10**100,10**100,-2**63,2**63-1]: assert oracle(*backward(n))==n
rng=random.Random(17)
for _ in range(10000):
 y=rng.randrange(1,10000);m=rng.randrange(1,13);d=1
 assert oracle(y,m,d)==(date(y,m,d)-date(1970,1,1)).days
src='''fd = (a: Int, b: Int) -> Int { { a % b < 0 } |> { True -> a / b - 1 False -> a / b } }
day = (y: Int, m: Int, d: Int) -> Int {
 y1 = { m <= 2 } |> { True -> y - 1 False -> y }
 era = fd(y1,400)
 yoe = y1 - era * 400
 mp = { m > 2 } |> { True -> m - 3 False -> m + 9 }
 doy = fd(153 * mp + 2,5) + d - 1
 era * 146097 + 365 * yoe + fd(yoe,4) - fd(yoe,100) + doy - 719468
}
civil = (n: Int) -> (Int,Int,Int) {
 z = n + 719468
 era = fd(z,146097)
 doe = z - era * 146097
 yoe = fd(doe - fd(doe,1460) + fd(doe,36524) - fd(doe,146096),365)
 y = yoe + era * 400
 doy = doe - { 365 * yoe + fd(yoe,4) - fd(yoe,100) }
 mp = fd(5 * doy + 2,153)
 d = doy - fd(153 * mp + 2,5) + 1
 m = { mp < 10 } |> { True -> mp + 3 False -> mp - 9 }
 yr = { m <= 2 } |> { True -> y + 1 False -> y }
 (yr,m,d)
}
@test
boundaries = () {
'''
for y,m,d in rows:
 n=oracle(y,m,d);src+=f' assert(day({y},{m},{d}) == {n})\n assert(civil({n}) == ({y},{m},{d}))\n'
src+=''' assert(-5 / 2 == -2)
 assert(5 / -2 == -2)
 assert(-5 / -2 == 2)
 assert(-1 % 86400000000000 == -1)
 assert(fd(-1,86400000000000) == -1)
 assert(-1 - fd(-1,86400000000000) * 86400000000000 == 86399999999999)
 assert(9223372036854775807 + 1 == 9223372036854775808)
 assert(-{-9223372036854775808} == 9223372036854775808)
 Nil
}
'''
(root/'test/main_test.i').write_text(src)
report=[]
for x in [(-400,2,29),(-100,3,1),(-1,1,1),(0,1,1),(0,2,29),(1,1,1),(1900,3,1),(2000,3,1),(10000,1,1)]: report.append(f'{x}: day={oracle(*x)}, weekday Monday=0: {(oracle(*x)+3)%7}')
for n in [-10**100,10**100,-2**63,2**63-1]:report.append(f'day {n}: {backward(n)}')
report.append(f'Passed 38424 signed-year boundary cases, 146098 consecutive inverse days, 10000 datetime comparisons; generated {len(rows)*2} i calendar assertions.')
(root/'results.txt').write_text('\n'.join(report)+'\n');print('\n'.join(report))
