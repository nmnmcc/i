(
	assertion = i/assertion
)
pair = (
	"b",
	() {
		("a", _) = pair
		1
	},
)

main = () {
	assertion.catch(pair.1) |>
		(Error(_)) { () }
		& (Ok(_)) { panic("missed Assert") }
}
