// spec: 9.4, 11.7
// expect: stop "control: continuation was already used"
// at: spec/main:15:4
// at: spec/main:12:2
(
	i/control
)

// §9.4 rule 5: `resume` and `cancel` together may be used one time.
@main
main = () {
	control.boundary((b) {
		control.suspend(b, (k) {
			control.resume(k, Nil)
			control.resume(k, Nil)
			Nil
		})
	})
	Nil
}
