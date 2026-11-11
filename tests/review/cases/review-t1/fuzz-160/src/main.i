f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "a",_) -> 0
(_: String - "ab" - "b",_) -> 1
("a" <> _,False) -> 2
(_: "a" | "b",True) -> 3
} }

main = () {
assert(f(("a",True)) == 1)
assert(f(("a",False)) == 1)
assert(f(("ab",True)) == 0)
assert(f(("ab",False)) == 0)
assert(f(("b",True)) == 0)
assert(f(("b",False)) == 0)
assert(f(("c",True)) == 0)
assert(f(("c",False)) == 0)
assert(f(("",True)) == 0)
assert(f(("",False)) == 0)
Nil
}
