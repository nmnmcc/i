(
	choose
	Target(..)
	Fs(..)
	Js(..)
	Wasi(..)
)
Js = ()
Wasi = ()
JsTarget = (Js)
NodeJS = (Js)
WasiTarget = (Wasi)
Native = ()
Host = ()
Target = JsTarget | NodeJS | WasiTarget | Native | Host
Fs = (read_file = (String) <- (Fail(String)) -> BitArray)
read_file = (path) <- (fs: Fs) { fs.read_file(path) }
choose = () <- (t: Target) {
	t |>
		(NodeJS(_)) { read_file("settings.txt") }
		| (Native) { read_file("settings.txt") }
		| (_) { <<>> }
}
