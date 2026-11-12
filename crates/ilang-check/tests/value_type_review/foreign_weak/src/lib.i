(
	Codec
	size
	Local
)
Codec(D, E) = (decode = (String) -> D, encode = (E) -> String)
make = <D, E>(d: (String) -> D, e: (E) -> String) {
	Codec(decode = d, encode = e)
}
size = make((s: String) -> Int { 1 }, (n: Int) -> String { "" })
Codec(Local, _) = size
