(
	app/secret
)

read = secret.raw.read

@main
main = () { assert(read() == 42) }
