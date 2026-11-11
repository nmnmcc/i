f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,True) -> 0
(_: String - "a",True) -> 1
(_: String - "ab" - "b",False) -> 2
("b",False) -> 3
(_: String - "a",_) ? False -> 4
("a" <> _,_) -> 5
("b",False) -> 6
(_: String - "ab" - "b",_) -> 7
} }

main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 2)
assert(f(("ab",True)) == 0)
assert(f(("ab",False)) == 5)
assert(f(("b",True)) == 0)
assert(f(("b",False)) == 3)
assert(f(("c",True)) == 0)
assert(f(("c",False)) == 2)
assert(f(("",True)) == 0)
assert(f(("",False)) == 2)
Nil
}
