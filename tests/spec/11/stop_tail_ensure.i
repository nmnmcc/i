// spec: 11.7, 9.5
// expect: stop "panic: body"
// at: spec/main:10:41
// at: spec/main:14:2
(
	i/resource
)

// A call of `resource.ensure` in tail position counts as a tail call.
guarded = () { resource.ensure(() { 1 + panic("body") }, () { Nil }) }

main = () {
	guarded()
	Nil
}
