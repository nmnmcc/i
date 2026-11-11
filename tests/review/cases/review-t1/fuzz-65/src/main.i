f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: "a" | "b",True) -> 0
(_,True) ? False -> 1
(_,_) -> 2
("a",True) -> 3
("ab",_) -> 4
("a" <> _,True) -> 5
} }

main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 2)
assert(f(("ab",True)) == 2)
assert(f(("ab",False)) == 2)
assert(f(("b",True)) == 0)
assert(f(("b",False)) == 2)
assert(f(("c",True)) == 2)
assert(f(("c",False)) == 2)
assert(f(("",True)) == 2)
assert(f(("",False)) == 2)
Nil
}
