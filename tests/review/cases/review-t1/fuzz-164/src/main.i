f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",False) -> 0
(_: String - "ab" - "b",False) -> 1
(_: String - "a",_) -> 2
("a",_) -> 3
("a" <> _,False) -> 4
("ab",_) -> 5
(_: String - "ab" - "b",_) -> 6
} }
@main
main = () {
assert(f(("a",True)) == 3)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 2)
assert(f(("ab",False)) == 2)
assert(f(("b",True)) == 2)
assert(f(("b",False)) == 2)
assert(f(("c",True)) == 2)
assert(f(("c",False)) == 0)
assert(f(("",True)) == 2)
assert(f(("",False)) == 0)
Nil
}
