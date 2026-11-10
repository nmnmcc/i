f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("ab",False) -> 0
(_: String - "ab" - "b",False) -> 1
("ab",_) -> 2
(_: "a" | "b",False) -> 3
("a" <> _,_) -> 4
("b",_) -> 5
(_,True) -> 6
} }
@main
main = () {
assert(f(("a",True)) == 4)
assert(f(("a",False)) == 1)
assert(f(("ab",True)) == 2)
assert(f(("ab",False)) == 0)
assert(f(("b",True)) == 5)
assert(f(("b",False)) == 3)
assert(f(("c",True)) == 6)
assert(f(("c",False)) == 1)
assert(f(("",True)) == 6)
assert(f(("",False)) == 1)
Nil
}
