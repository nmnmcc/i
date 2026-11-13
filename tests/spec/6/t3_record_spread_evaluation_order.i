// spec: 6.2, 6.7
// expect: stop "panic: base"
// at: spec/main:9:35
// at: spec/main:12:5
(
	main
)

base = () -> (a = Int, b = Int) { panic("base") }

main = () {
	(..base(), a = panic("field"))
	Nil
}
