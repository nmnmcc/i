// §4.4: `{ A | B } & C` is `A & C | B & C`. A written intersection with a
// union in it is worked out, so the solver does not go round forever.
Box(A) = (item = A)

unbox = <A>(b : Box(A)) -> A { b.item }

call = (x : { Box(Int) | () } & Box(Int)) { unbox(x) }

bind = (x : { Box(Int) | () } & Box(Int)) {
	y : Box(_) = x
	y
}

field = (x : { Box(Int) | () } & Box(Int)) { x.item }

with_var = <A>(x : A & { Box(Int) | () }) {
	x |> (Box(item = item)) { item } & (()) { 0 }
}

empty = (x : { Box(Int) | () } & String) { x }
