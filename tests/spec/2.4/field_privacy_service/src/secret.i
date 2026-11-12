(
	Secret
	Raw
	value
	raw
)

Secret {
	Secret(number: Int)
}

Raw {
	Raw(read: () -> Int)
}

value = Secret(number: 42)

raw = Raw(read: () { 42 })
