(
	list = std/list
	(Some, None) = std/option
)

value = list.fold([1, 2], None, (acc, x) {
	acc |> {
		Some(p) -> Some((p.0 + 1, x))
		None    -> Some((0, x))
	}
})
