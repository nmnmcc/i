// literal-types §5.3: one deferred environment per binding, before generalization.
pair = (
	"b",
	() {
		("a", _) = pair
		1
	},
)

identity_pair = (
	(x) { x },
	() {
		"a" = identity_pair.0("b")
		1
	},
)

total_pair = (
	(1, 2),
	() {
		(a, b) = total_pair.0
		a
	},
)

f = (s : String) -> Int {
	(a, b) = (1, 2)
	{
		"a" = s
		a
	} <- (Assert(fail = (_) { panic("own") }))
}

s = "b"

top = {
	() {
		"a" = s
		1
	}
}()

written = () -> Int {
	"a" = s
	1
}
