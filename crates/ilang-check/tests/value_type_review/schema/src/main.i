Schema(D, E) = (decode = (String) -> D, encode = (E) -> String)
make = <D, E>(d: (String) -> D, e: (E) -> String) {
	Schema(decode = d, encode = e)
}
user = make(
	(s: String) -> (String, Int) { (s, 1) },
	(v: (String, Int)) -> String { v.0 },
)
Schema(User, _) = user
use = (u: User) { u.0 }
result = use(("a", 2))
