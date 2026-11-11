f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: "a" | "b",False) -> 0
(_,_) -> 1
("a",True) -> 2
(_: String - "ab" - "b",_) -> 3
("a",False) -> 4
(_: String - "ab" - "b",_) -> 5
} }

main = () {
assert(f(("a",True)) == 1)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 1)
assert(f(("ab",False)) == 1)
assert(f(("b",True)) == 1)
assert(f(("b",False)) == 0)
assert(f(("c",True)) == 1)
assert(f(("c",False)) == 1)
assert(f(("",True)) == 1)
assert(f(("",False)) == 1)
Nil
}
