// spec: 7.3
// expect: stop "panic: caught group local"

// literal-types §5.3: deferred fitted failure in its own effect frame.
pair = (
	"b",
	() {
		g = () {
			("a", _) = pair
			1
		}
		g()
	},
)

main = () { pair.1() <- (Assert(fail = (_) { panic("caught group local") })) }
