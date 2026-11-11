// spec: 11.7, 9.3
// expect: stop "panic: boom"
// at: spec/main:10:33
// at: spec/main:14:2
(
	i/fail
)

// A call of `fail.catch` in tail position counts as a tail call: it is gone.
safe = () { fail.catch(() { 1 + panic("boom") }) }

@main
main = () {
	safe()
	Nil
}
