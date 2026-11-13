// spec: 7.4
// expect: pass
all = (s: String) -> String { s |> ("" <> r) { r } }
main = () {
	assert(all("i") == "i")
	Nil
}
