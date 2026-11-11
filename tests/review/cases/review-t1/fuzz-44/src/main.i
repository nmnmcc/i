f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a",False) -> 0
(_: String - "a",_) -> 1
(_: "a" | "b",_) -> 2
("a",_) ? False -> 3
("a",False) -> 4
(_,_) -> 5
(_,_) -> 6
("b",True) ? False -> 7
("b",True) -> 8
} }

main = () {
assert(f(("a",True)) == 2)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 1)
assert(f(("ab",False)) == 1)
assert(f(("b",True)) == 1)
assert(f(("b",False)) == 1)
assert(f(("c",True)) == 1)
assert(f(("c",False)) == 1)
assert(f(("",True)) == 1)
assert(f(("",False)) == 1)
Nil
}
