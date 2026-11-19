// spec: 11.3
// expect: stop "float: result is not finite"

// A NaN cannot become a Float value: the language's Float is always finite,
// so no NaN ever reaches equality. Constructing one from its bits stops.
(
	i/core
)

main = () {
	core.float_from_bits(9221120237041090560)
}
