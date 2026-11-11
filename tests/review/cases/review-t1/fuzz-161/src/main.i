f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,_) ? False -> 0
(_,_) -> 1
("b",_) -> 2
(_: String - "a",True) -> 3
("b",False) ? False -> 4
(_: "a" | "b",False) -> 5
(_,False) -> 6
(_: String - "a",True) -> 7
("a",True) -> 8
} }

main = () {
assert(f(("a",True)) == 1)
assert(f(("a",False)) == 1)
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
