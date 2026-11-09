import (
	hello/main
)

@test
repeats = () {
	case main.repeat("a", 3, "-") {
		"a-a-a" -> Nil
		other   -> panic(other)
	}
}

@test
greets_by_label = () {
	case main.greet(main.Greeting(times: 1, name: "i")) {
		"hello i!" -> Nil
		other      -> panic(other)
	}
}
