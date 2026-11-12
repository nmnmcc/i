(
	Secret
	Pack
	Choice
	Left
	Right
	Raw
	Open(..)
	make
	read
	local_update
	local_choice
	local_known
	callback
	raw
	open
)

Pack(A) = (value = A, parts = (A, Int), read = () -> A)

Secret(A) = Pack(A)
Left = (value = Int)
Right = (value = Int)
Choice = Left | Right

Raw = (read = () -> Int)

Open = (value = Int)
Other = (value = Int)
Open = Open | Other

make = (value) { Pack(value = value, parts = (value, 7), read = () { value }) }

read = <A>(value: Secret(A)) { value.value }

local_update = <A>(value: Secret(A), next: A) { Pack(..value, value = next) }

local_choice = (value: Choice) { value.value }

local_known = Pack(value = 42, parts = (42, 7), read = () { 42 }).value

callback = <A>(value: Secret(A)) { value.read }

raw = Raw(read = () { 42 })

open = Open(value = 11)
