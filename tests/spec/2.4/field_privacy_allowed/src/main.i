(
	app/secret
	app/facade
)

id = <A>(value : A) { value }

pass = <A>(value : secret.Secret(A)) { id(value) }

public = (value : facade.OpenAlias) { value.value }

allowed = () {
	value = pass(facade.value)
	assert(secret.read(value) == 42)
	assert(
		secret.read(secret.local_update(value, 23)) == 23,
	)
	assert(
		secret.read(
			pass(secret.make("ok")),
		) ==
			"ok",
	)
	assert(public(facade.open) == 11)
	assert(
		{
			facade.open |>
				(secret.Open(..) = v) { secret.Open(..v, value = 29).value }
				| (_) { 0 }
		} ==
			29,
	)
	assert((value, 5).1 == 5)
	assert(
		secret.read(facade.constructor(19)) == 19,
	)
	assert(secret.local_known == 42)
	assert(secret.callback(value)() == 42)
	assert(secret.Other(value = 17).value == 17)
	assert(
		{
			facade.open |>
				(secret.Open(value = value)) { value }
				| (secret.Other(value = value)) { value }
		} ==
			11,
	)
}
