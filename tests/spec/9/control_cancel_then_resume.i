// spec: 9.4, 11.7
// expect: stop "control: continuation was already used"
(
	i/control
)

@main
main = () {
	control.boundary((b) {
		control.suspend(b, (k) {
			control.cancel(k)
			control.resume(k, Nil)
		})
	})
}
