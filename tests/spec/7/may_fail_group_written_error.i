// spec: 7.3
// expect: compile-error E0501

// literal-types §5.3: deferred fitted failure in its own effect frame.
s = "b"

f = () -> Int {
	"a" = s
	1
}
