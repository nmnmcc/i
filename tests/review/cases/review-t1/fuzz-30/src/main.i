f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a",_) -> 0
(_: String - "a",True) -> 1
("ab",_) -> 2
(_,True) ? False -> 3
("a",True) ? False -> 4
(_: "a" | "b",_) -> 5
(_: "a" | "b",True) ? False -> 6
(_: String - "a",_) -> 7
("b",_) -> 8
} }
@main
main = () {
assert(f(("a",True)) == 0)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 1)
assert(f(("ab",False)) == 2)
assert(f(("b",True)) == 1)
assert(f(("b",False)) == 5)
assert(f(("c",True)) == 1)
assert(f(("c",False)) == 7)
assert(f(("",True)) == 1)
assert(f(("",False)) == 7)
Nil
}
