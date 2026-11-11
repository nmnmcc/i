f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: "a" | "b",_) -> 0
("a" <> _,False) -> 1
(_: String - "ab" - "b",False) -> 2
(_: String - "a",True) -> 3
} }

main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 3)
assert(f(("ab",False)) == 1)
assert(f(("b",True)) == 0)
assert(f(("b",False)) == 0)
assert(f(("c",True)) == 3)
assert(f(("c",False)) == 2)
assert(f(("",True)) == 3)
assert(f(("",False)) == 2)
Nil
}
