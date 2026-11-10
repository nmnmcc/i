f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "a",True) -> 0
(_: "a" | "b",False) -> 1
("a" <> _,False) -> 2
(_: String - "ab" - "b",False) -> 3
(_: String - "a",False) -> 4
("a" <> _,False) -> 5
("ab",False) -> 6
(_,True) -> 7
} }
@main
main = () {
assert(f(("a",True)) == 7)
assert(f(("a",False)) == 1)
assert(f(("ab",True)) == 0)
assert(f(("ab",False)) == 2)
assert(f(("b",True)) == 0)
assert(f(("b",False)) == 1)
assert(f(("c",True)) == 0)
assert(f(("c",False)) == 3)
assert(f(("",True)) == 0)
assert(f(("",False)) == 3)
Nil
}
