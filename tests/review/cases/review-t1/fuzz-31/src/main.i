f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,_) -> 0
("b",True) -> 1
("a",True) -> 2
(_: String - "a",False) -> 3
(_: "a" | "b",False) -> 4
} }
@main
main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 0)
assert(f(("ab",False)) == 0)
assert(f(("b",True)) == 0)
assert(f(("b",False)) == 0)
assert(f(("c",True)) == 0)
assert(f(("c",False)) == 0)
assert(f(("",True)) == 0)
assert(f(("",False)) == 0)
Nil
}
