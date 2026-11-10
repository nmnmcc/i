f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a" <> _,True) -> 0
("b",_) ? False -> 1
(_: String - "a",False) -> 2
("b",True) -> 3
(_: String - "ab" - "b",True) -> 4
(_: "a" | "b",_) -> 5
(_,False) -> 6
} }
@main
main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 5)
assert(f(("ab",True)) == 0)
assert(f(("ab",False)) == 2)
assert(f(("b",True)) == 3)
assert(f(("b",False)) == 2)
assert(f(("c",True)) == 4)
assert(f(("c",False)) == 2)
assert(f(("",True)) == 4)
assert(f(("",False)) == 2)
Nil
}
