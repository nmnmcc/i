(
	math = app/math
)

(
	main
)

main = () {
	m = math.make_math()
	assert(m.add(2, 3) == 5)
}
