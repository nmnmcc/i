(
	Schema
	make
)
Schema(D, E) = (decode = (String) -> D, encode = (E) -> String)
make = <D, E>(d: (String) -> D, e: (E) -> String) {
	Schema(decode = d, encode = e)
}
