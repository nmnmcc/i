empty = core.never
f = (n) { n }
g = (0) { 1 } | (n ? n > 0) { 2 } | (_) { 3 }
match = hs |> ((name, _)) { (x) { log(name, x) } } | (_) { (x) { x } }
a = {
	go = (n) -> Int { n + 1 }
	go(1)
}
b = {
	go = (n) -> Int { step(n) }
	go(1)
}
c = {
	k: (Int) -> Int = f
	k(1)
}
d = {
	go = <A>(x: A) -> A { x }
	go(1)
}
e = {
	go = (n) <- (log: Log) -> Int {
		log.write(n)
		n
	}
	go(1)
}
h = {
	k = 2
	g = (x) -> Int { x * k }
	g(1)
}
i = ((0, _) = t) { t } | (_) { 0 }
j = (Ok(v) = r) { v } | (_) { 0 }
k = ("ab" <> rest = s) { s } | (_) { "" }
l = ((a, b) = p) { () { log(p) } }
m = (f: (Int) -> Int ? ok) { f(1) } | (_) { 0 }
