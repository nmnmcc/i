(
	Codec
	size
)

Codec(D, E) {
	Codec(decode: (String) -> D, encode: (E) -> String)
}

make(D, E) = (d: (String) -> D, e: (E) -> String) {
	Codec(decode: d, encode: e)
}

id = (x) { x }

size = id(make((s: String) -> Int { 1 }, (n: Int) -> String { "" }))

Codec(LocalSize, _) = size
