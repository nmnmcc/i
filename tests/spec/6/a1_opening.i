// spec: opening, 6.1, 7.1
// expect: pass
(
	main
)
make = (n) { (x) -> Int { x + n } }
read = (v : { Int | String }) { v |> (x : Int) { x } | (_) { 0 } }

main = () {
	assert(make(4)(3) == 7)
	assert(read(3) == 3)
	assert(read("other") == 0)
	Nil
}
