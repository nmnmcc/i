f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a" <> _,True) -> 0
("a" <> _,False) -> 1
("a" <> _,True) -> 2
("a",True) ? False -> 3
("a",True) -> 4
(_: String - "ab" - "b",_) -> 5
("a",True) -> 6
(_,_) -> 7
(_: "a" | "b",True) -> 8
} }
@main
main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 1)
assert(f(("ab",True)) == 0)
assert(f(("ab",False)) == 1)
assert(f(("b",True)) == 7)
assert(f(("b",False)) == 7)
assert(f(("c",True)) == 5)
assert(f(("c",False)) == 5)
assert(f(("",True)) == 5)
assert(f(("",False)) == 5)
Nil
}
