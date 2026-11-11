(
	app/secret
)

read = secret.raw.read


main = () { assert(read() == 42) }
