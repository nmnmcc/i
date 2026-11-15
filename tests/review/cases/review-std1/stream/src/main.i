import(probe / stream)
Bias = (value = Int)
source = () -> stream.Stream(Int, Never, Bias) {
	stream.make(() <- (b : Bias) { stream.Next(b.value, stream.new()) })
}

main = () {
	assert(
		stream.to_list(source()) == [10],
	) <- (Bias(value = 10))
}
