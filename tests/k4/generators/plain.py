# Adapted from the independent K4 review generator.
import random
import re
HEAD = 'Dep { Dep(v: Int) }\nget = (d ! Dep) { d.v }\nSvc(A) { Svc(f: () -> A, m: (A) -> Nil) }\nask(A) = (s ! Svc(A)) { s.f() }\nCov(A) { Cov(f: () -> A) }\naskc(A) = (s ! Cov(A)) { s.f() }\nCb(N) { Cb(f: () -> Int ! N, m: (N) -> Nil) }\ncall(N) = (c: Cb(N)) {\n  Cb(f, ..) = c\n  f()\n}\nrun(A, N) = (body: () -> A ! N) { body() }\ntwice(N) = (body: () -> Int ! N) { body() + body() }\nwith_int = (body) {\n  ! Svc(Int) = Svc(() { 1 }, (_: Int) { Nil })\n  body()\n}\nwith_dep = (body) {\n  ! Dep = Dep(5)\n  body()\n}\nwith_cov = (body, v) {\n  ! Cov(_) = Cov(() { v })\n  body()\n}\npick = (a, b) { [a, b] }\n'
ATOMS = ['1', 'get()', 'ask()', 'askc()', 'h0()', 'h1()', 'run(() { ask() })', 'twice(() { get() })', 'with_int(() { ask() })', 'with_dep(() { get() })', 'with_cov(() { askc() }, 3)', 'call(Cb(() { get() }, (_: Dep) { Nil }))', 'call(Cb(() { ask() }, (_) { Nil }))', 'with_int(() { run(() { ask() + 1 }) })', 'with_dep(() { with_int(() { get() + ask() }) })', 'run(h0)', 'twice(h1)']
ROWS = ['Int', 'String', 'Dep', 'Never', '_']

def e():
    return ' + '.join((R.choice(ATOMS) for _ in range(R.randint(1, 3))))

def st():
    c = R.random()
    if c < 0.2:
        return '! Dep = Dep(1)'
    if c < 0.45:
        return '! Svc(Int) = Svc(() { ' + e() + ' }, (_: Int) { Nil })'
    if c < 0.55:
        return '! Svc(String) = Svc(() { "s" }, (_: String) { Nil })'
    if c < 0.7:
        return '! Cov(Int) = Cov(() { ' + e() + ' })'
    if c < 0.8:
        return f'l{R.randint(0, 9)} = () {{ {e()} }}'
    if c < 0.9:
        return f'p{R.randint(0, 9)} = pick(() {{ {e()} }}, () {{ {e()} }})'
    return f'x{R.randint(0, 9)} = {e()}'

def sources(seed, count):
    global R
    R = random.Random(seed)
    for i in range(count):
        hs = [f'h0 = () {{ {R.choice(ATOMS[:4] + ATOMS[6:16])} }}', f'h1 = () {{\n  ' + '\n  '.join([st() for _ in range(R.randint(0, 2))] + [R.choice(ATOMS[:5] + ATOMS[6:16])]) + '\n}']
        ms = ['! Dep = Dep(1)', '! Svc(Int) = Svc(() { get() }, (_: Int) { Nil })', '! Cov(Int) = Cov(() { 2 })']
        R.shuffle(ms)
        ms = ms[:R.randint(1, 3)]
        for _ in range(R.randint(0, 4)):
            ms.insert(R.randrange(len(ms) + 1), st())
        used = [m.split(' = ')[0] for m in ms if re.match('^[lx]\\d = ', m)]
        fin = e() + ''.join((' + ' + (u + '()' if u[0] == 'l' else u) for u in sorted(set(used))))
        src = HEAD + '\n'.join(hs) + '\n@main\nmain = () {\n  ' + '\n  '.join(ms + [fin]) + '\n  Nil\n}\n'
        yield src
