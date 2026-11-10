f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "a",_) -> 0
(_,_) -> 1
(_: String - "ab" - "b",False) -> 2
} }
@main
main = () {
assert(f(("a",True)) == 1)
assert(f(("a",False)) == 1)
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
