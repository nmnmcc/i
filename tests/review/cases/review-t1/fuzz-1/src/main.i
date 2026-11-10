f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: "a" | "b",False) -> 0
(_: String - "a",False) -> 1
(_,_) -> 2
(_,True) -> 3
("b",_) -> 4
(_,True) -> 5
("a",_) -> 6
("b",False) -> 7
} }
@main
main = () {
assert(f(("a",True)) == 2)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 2)
assert(f(("ab",False)) == 1)
assert(f(("b",True)) == 2)
assert(f(("b",False)) == 0)
assert(f(("c",True)) == 2)
assert(f(("c",False)) == 1)
assert(f(("",True)) == 2)
assert(f(("",False)) == 1)
Nil
}
