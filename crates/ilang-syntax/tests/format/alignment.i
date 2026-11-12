User = (
	name = String,
	age = Int,
	email_address = String,
	is_administrator = Bool,
	created = Int,
)
Guest = ()
User = User | Guest

make = () {
	user = User(
		name = "a long name for the user",
		age = 42,
		email_address = "someone@example.com",
		is_administrator = False,
		created = 0,
	)
	short = 1 // one
	a_longer_name = 2 // two

	after_blank = 3 // three
	user
}

Point = (x = Int, y = Int)
