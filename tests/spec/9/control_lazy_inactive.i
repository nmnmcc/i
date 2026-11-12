// spec: 9.4, 11.7
// expect: stop "control: boundary is not active"
// at: spec/main:15:33
// at: spec/main:12:2
(
	i/control
)

// `inner` is paused inside `k`, and is active again after the resume. The
// `f` of a suspend to it is done in its place, where it is not active.
main = () {
	control.boundary((outer) {
		control.boundary((inner) {
			control.suspend(outer, (k) { control.resume(k, Nil) })
			control.suspend(inner, (k) { control.suspend(inner, (k2) { 1 }) })
			0
		})
	})
	Nil
}
