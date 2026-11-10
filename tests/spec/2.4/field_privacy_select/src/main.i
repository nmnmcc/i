(
	app/secret
)

@main
main = () { assert(secret.value.number == 42) }
