f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a" <> _,_) -> 0
(_: String - "ab" - "b",True) -> 1
(_: String - "a",True) -> 2
("a",True) ? False -> 3
(_,False) -> 4
("a" <> _,_) -> 5
("a" <> _,True) -> 6
} }
@main
main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 0)
assert(f(("ab",False)) == 0)
assert(f(("b",True)) == 2)
assert(f(("b",False)) == 4)
assert(f(("c",True)) == 1)
assert(f(("c",False)) == 4)
assert(f(("",True)) == 1)
assert(f(("",False)) == 4)
Nil
}
