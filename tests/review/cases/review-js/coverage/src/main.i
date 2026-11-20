Js = (Int)
NodeJS = (Int)
Wasi = (Int)
Native = (Int)
Host = ()
Target = Js | NodeJS | Wasi | Native | Host
f = (t : Target) {
	case
	t
	(Js(j)) { () } & (Wasi(w)) { () } & (Host()) { () }
}

main = () { f(Host()) }
