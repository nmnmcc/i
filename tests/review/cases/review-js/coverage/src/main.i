Js = (Int)
NodeJS = (Int)
Wasi = (Int)
Native = (Int)
Host = ()
Target = Js | NodeJS | Wasi | Native | Host
f = (t : Target) {
	case
	t
	(Js(j)) { Nil } | (Wasi(w)) { Nil } | (Host) { Nil }
}

main = () { f(Host) }
