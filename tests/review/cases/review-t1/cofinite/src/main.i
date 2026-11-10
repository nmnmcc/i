g = (s: String) { s |> {
 _: String - "a" - "b" -> 3
 "a" -> 1
 r -> { x: "b" = r  2 }
} }
h = (n: Int | String) { n |> {
 _: String - "a" -> 1
 "a" -> 2
 _: Int - 0 -> 3
 0 -> 4
} }
@main
main = () {
 assert(g("b")==2) assert(g("z")==3)
 assert(h(0)==4) assert(h(1073741824)==3)
 assert(h(-1073741825)==3) assert(h(1000000000000000000000000000000)==3)
 assert(h("b")==1) Nil
}
