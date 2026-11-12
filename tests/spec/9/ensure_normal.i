// spec: 9.5, 11.7
// expect: stop "panic: final"
// at: spec/main:8:14
(
	i/resource
)

final = () { panic("final") }

// `with` runs after `body` ends normally. The call of `resource.ensure` is
// in tail position, so it counts as a tail call: its position is gone
// (§11.7).
main = () { resource.ensure(() { 1 }, final) }
