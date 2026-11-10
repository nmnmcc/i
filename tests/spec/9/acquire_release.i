// spec: 9.5
// expect: stop "panic: released 5"
(
	i/fail
	i/resource
)

// `release` gets the resource when `use` ends with an error.
@main
main = () {
	fail.catch(() {
		resource.acquire(() { 5 }, (r) {
			r |> {
				5 -> panic("released 5")
				_ -> Nil
			}
		}, (r) { fail.raise(r) })
	})
}
