S(N) {
	S(() -> Nil ! N)
}

use(N) = (s ! S(N)) -> Nil ! N | S(N) {
	S(f) = s
	f()
}

example = () { use() }


main = () { example() }
