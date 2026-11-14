(
	choose
	Target(..)
	TargetKind(..)
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
TargetKind = JsTarget | NodeJS | WasiTarget | Native | Host
Target = (TargetKind)
Fs = (read_file = (String) <- (Fail(String)) -> BitArray)
read_file = (path) <- (fs : Fs) { fs.read_file(path) }
choose = () <- (t : Target) {
	t |>
		(Target(NodeJS(_))) { read_file("settings.txt") }
		| (Target(Native)) { read_file("settings.txt") }
		| (_) { <<>> }
}
