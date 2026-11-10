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

value = Secret(42)

raw = Raw(() { 42 })
