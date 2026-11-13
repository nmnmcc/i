import(probe / stream)
Bias = (value = Int)
end_bias = () -> stream.Stream(Int, Never, Bias) {
	stream.make(() -> stream.Step(Int, Never, Bias) { stream.Done })
}
source = () -> stream.Stream(Int, Never, Bias) {
	stream.make(() <- (b: Bias) { stream.Next(b.value, end_bias()) })
}

main = () {
	s = source()
	{ assert(stream.to_list(s) == [10]) <- (Bias(value = 10)) }
	{ assert(stream.to_list(s) == [20]) <- (Bias(value = 20)) }
	adapted = stream.map_environment(s, (
		body: () <- (Bias) -> stream.Step(Int, Never, Bias)
	) { body() <- (Bias(value = 30)) })
	assert(stream.to_list(adapted) == [30])
}
