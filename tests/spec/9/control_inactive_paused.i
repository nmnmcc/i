// spec: 9.4, 11.7
// expect: stop "control: boundary is not active"
// at: spec/control_inactive_paused:11:30
// at: spec/control_inactive_paused:10:3
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
