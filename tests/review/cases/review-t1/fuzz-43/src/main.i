f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,_) -> 0
("a",False) -> 1
(_: String - "a",True) -> 2
(_: String - "ab" - "b",False) -> 3
("b",True) ? False -> 4
(_: String - "ab" - "b",False) ? False -> 5
("ab",_) -> 6
(_: String - "ab" - "b",_) -> 7
} }

main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 0)
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
