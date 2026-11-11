// spec: 9.4, 11.7
// expect: stop "control: boundary is not active"
// at: spec/control_inactive_after:12:8
(
	i/control
)

// A boundary is not active after its body has ended (§9.4 rule 1).

main = () {
	later = control.boundary((b) {
		() { control.suspend(b, (k) { () { Nil } }) }
	})
	later()
}
