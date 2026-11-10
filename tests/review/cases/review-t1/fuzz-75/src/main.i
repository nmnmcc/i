f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a",False) -> 0
(_,True) -> 1
("a" <> _,True) -> 2
(_: String - "a",False) -> 3
("a",False) -> 4
(_: "a" | "b",True) -> 5
(_: String - "ab" - "b",_) -> 6
(_: "a" | "b",_) -> 7
("a",_) -> 8
} }
@main
main = () {
assert(f(("a",True)) == 1)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 1)
assert(f(("ab",False)) == 3)
assert(f(("b",True)) == 1)
assert(f(("b",False)) == 3)
assert(f(("c",True)) == 1)
assert(f(("c",False)) == 3)
assert(f(("",True)) == 1)
assert(f(("",False)) == 3)
Nil
}
