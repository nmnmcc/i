// spec: 9.4, 11.7
// expect: stop "control: boundary is not active"
// at: spec/control_inactive_paused:13:28
// at: spec/control_inactive_paused:12:2
(
	i/control
)

// A boundary is not active while its body is paused in a continuation.
@main
main = () {
	control.boundary((b) {
		control.suspend(b, (k) { control.suspend(b, (k2) { Nil }) })
	})
	Nil
}
