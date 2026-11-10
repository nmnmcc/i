(
	app/secret
)

(
	Alias
	OpenAlias
	value
	raw
	open
	constructor
)

Alias(A) = secret.Secret(A)

OpenAlias = secret.Open

value = secret.make(42)

raw = secret.raw

open = secret.open

constructor = secret.make
