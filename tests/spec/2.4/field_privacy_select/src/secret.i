(
	Secret
	Raw
	value
	raw
)

Secret = (number = Int)

Raw = (read = () -> Int)

value = Secret(number = 42)

raw = Raw(read = () { 42 })
