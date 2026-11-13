(
	app/secret
)

read = () <- (raw: secret.Raw) { raw.read() }

main = () { assert(read() == 42) <- (secret.raw) }
