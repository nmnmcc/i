f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "a",False) -> 0
("b",True) -> 1
(_: String - "a",_) -> 2
(_: String - "a",True) -> 3
(_: String - "ab" - "b",_) -> 4
} }
@main
main = () {
assert(f(("a",True)) == 4)
assert(f(("a",False)) == 4)
assert(f(("ab",True)) == 2)
assert(f(("ab",False)) == 0)
assert(f(("b",True)) == 1)
assert(f(("b",False)) == 0)
assert(f(("c",True)) == 2)
assert(f(("c",False)) == 0)
assert(f(("",True)) == 2)
assert(f(("",False)) == 0)
Nil
}
