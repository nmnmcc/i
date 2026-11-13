// Proposed T3 plus landed T2. A plain user-defined type and constructor.
Schema(A) = (decode = () -> A)
make = (c: Bool) {
	c |>
		(True) { Schema(decode = () { (a = 1) }) }
		| (False) { Schema(decode = () { (a = "x") }) }
}
schema = make(True)
Schema(Row) = schema
// Row denotes (a: 1) | (a: "x").
read = (r: Row) { r.a } // Both alternatives provide a.
read_inferred = (r) { r.a } // accepts schema.decode()
read_written = (r: (a = 1 | "x")) { r.a }
