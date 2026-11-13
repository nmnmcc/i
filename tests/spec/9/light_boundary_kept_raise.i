// spec: 9.2, 9.3, 9.4, 11.7
// expect: stop "control: boundary is not active"
// at: spec/main:14:31
(
	i/control
	i/fail
)

// A function kept inside a `catch` raises through the `Fail` of the catch
// after the catch has ended: the program stops, because the boundary is
// not active, at the same place as when the `Fail` value itself is used.
main = () {
	r: Result((Int) -> Nil, Int) = fail.catch(() {
		control.keep_env((x: Int) { fail.raise(x) })
	})
	r |> (Ok(k)) { k(1) } | (Error(_)) { Nil }
}
