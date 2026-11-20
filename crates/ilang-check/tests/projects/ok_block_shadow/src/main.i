(
	shadow
)

// A nested block, a branch and an arm are scopes of their own: each may
// bind a name that an outer block binds (§5).
shadow = (n : Int) {
	x = n
	y = {
		x = 2
		x
	}
	z = x |>
		(0) {
			x = 3
			x
		}
		& (_) {
			x = 4
			x
		}
	x + y + z
}
