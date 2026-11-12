(
	Secret
	Choice
	Left
	Right
	Raw
	Open
	OpenCase(..)
	Other(..)
	make
	read
	local_update
	local_choice
	local_known
	callback
	raw
	open
)

Secret(A) {
	Pack(value: A, parts: (A, Int), read: () -> A)
}

Choice = Left | Right

Left {
	Left(value: Int)
}

Right {
	Right(value: Int)
}

Raw {
	Raw(read: () -> Int)
}

Open = OpenCase | Other

OpenCase {
	Open(value: Int)
}

Other {
	Other(value: Int)
}

make = (value) { Pack(value: value, parts: (value, 7), read: () { value }) }

read(A) = (value: Secret(A)) { value.value }

local_update(A) = (value: Secret(A), next: A) { Pack(..value, value: next) }

local_choice = (value: Choice) { value.value }

local_known = Pack(value: 42, parts: (42, 7), read: () { 42 }).value

callback(A) = (value: Secret(A)) { value.read }

raw = Raw(read: () { 42 })

open: Open = Open(value: 11)
