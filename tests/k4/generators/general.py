# Adapted from the independent K4 review generator.
import random
import re
HEAD = 'Dep { Dep(v: Int) }\nget = (d ! Dep) { d.v }\nS(N) { S(f: () -> Int ! N, m: (N) -> Nil) }\nT(N) { T(f: () -> Int ! N, m: (N) -> Nil) }\nus(N) = (s ! S(N)) {\n  S(f, ..) = s\n  f()\n}\nut(N) = (t ! T(N)) {\n  T(f, ..) = t\n  f()\n}\nusa(N) = (s ! S(N)) -> Int ! N | S(N) {\n  S(f, ..) = s\n  f()\n}\nrun(A, N) = (body: () -> A ! N) { body() }\n'

def row(d=0):
    c = R.random()
    if d > 2 or c < 0.3:
        return 'Never'
    if c < 0.5:
        return 'Dep'
    if c < 0.65:
        return f'S({row(d + 1)})'
    if c < 0.8:
        return f'T({row(d + 1)})'
    a, b = (row(d + 1), row(d + 1))
    if a == 'Never':
        return b
    if b == 'Never' or a == b:
        return a
    return f'{a} | {b}'

def expr(helpers, d=0):
    n = R.randint(1, 3)
    parts = []
    for _ in range(n):
        c = R.random()
        if c < 0.15:
            parts.append('1')
        elif c < 0.3:
            parts.append('get()')
        elif c < 0.45:
            parts.append('us()')
        elif c < 0.55:
            parts.append('usa()')
        elif c < 0.7:
            parts.append('ut()')
        elif c < 0.85 and helpers:
            parts.append(R.choice(helpers) + '()')
        elif d < 2:
            parts.append('run(() { ' + expr(helpers, d + 1) + ' })')
        else:
            parts.append('2')
    return ' + '.join(parts)

def stmt(helpers, ind, d):
    c = R.random()
    if c < 0.25:
        return ind + '! Dep = Dep(1)'
    k = 'S' if c < 0.65 else 'T'
    r = row()
    return f'{ind}! {k}({r}) = {k}(() {{ {expr(helpers, d + 1)} }}, (_: {r}) {{ Nil }})'

def body(helpers, ind, d=0, locals_=None):
    out = []
    hs = list(helpers)
    for _ in range(R.randint(0, 3)):
        if R.random() < 0.3 and d < 2:
            name = f'l{d}_{len(out)}'
            out.append(f"{ind}{name} = () {{\n{body(hs, ind + '  ', d + 1)}\n{ind}}}")
            hs.append(name)
        else:
            out.append(stmt(hs, ind, d))
    out.append(ind + expr(hs, d))
    return '\n'.join(out)

def sources(seed, count):
    global R
    R = random.Random(seed)
    for i in range(count):
        helpers = []
        src = HEAD
        for h in range(R.randint(0, 3)):
            name = f'h{h}'
            src += f"{name} = () {{\n{body(helpers, '  ')}\n}}\n"
            helpers.append(name)
        src += f"@main\nmain = () {{\n{body(helpers, '  ')}\n  Nil\n}}\n"
        yield src
