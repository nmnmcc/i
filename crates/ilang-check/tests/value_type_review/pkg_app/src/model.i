(
	(Schema, make) = schema/api
)

(
	User
)
user = make(
	(s: String) -> (String, Int) { (s, 1) },
	(u: (String, Int)) -> String { u.0 },
)
Schema(User, _) = user
