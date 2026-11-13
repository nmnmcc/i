(
	(Schema, make) = review/lib
)
user = make(
	(s : String) -> (String, Int) { (s, 1) },
	(v : (String, Int)) -> String { v.0 },
)
Schema(User, _) = user
x : User = ("s", 1)
