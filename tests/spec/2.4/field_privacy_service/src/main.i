(
	app/secret
)

read = (raw ! secret.Raw) { raw.read() }


main = () {
	! secret.Raw = secret.raw
	assert(read() == 42)
}
