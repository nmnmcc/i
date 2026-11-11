Sequence(a) { Sequence(Tree(a)) }
Node(a) { Leaf(a) Branch2(Int, Node(a), Node(a)) Branch3(Int, Node(a), Node(a), Node(a)) }
Digit(a) { One(Node(a)) Two(Node(a), Node(a)) Three(Node(a), Node(a), Node(a)) Four(Node(a), Node(a), Node(a), Node(a)) }
Tree(a) { EmptyTree Single(Node(a)) Deep(Int, Digit(a), Tree(a), Digit(a)) }
map_node = (node: Node(a), with: (a) -> b ^ e ! n) -> Node(b) ^ e ! n {
 case node {
  Leaf(x) -> Leaf(with(x))
  Branch2(m, x, y) -> Branch2(m, map_node(x, with), map_node(y, with))
  Branch3(m, x, y, z) -> Branch3(m, map_node(x, with), map_node(y, with), map_node(z, with))
 }
}
map_digit = (digit: Digit(a), with: (a) -> b ^ e ! n) -> Digit(b) ^ e ! n {
 case digit {
  One(x) -> One(map_node(x, with))
  Two(x,y) -> Two(map_node(x, with), map_node(y, with))
  Three(x,y,z) -> Three(map_node(x, with), map_node(y, with), map_node(z, with))
  Four(w,x,y,z) -> Four(map_node(w, with), map_node(x, with), map_node(y, with), map_node(z, with))
 }
}
map_tree = (tree: Tree(a), with: (a) -> b ^ e ! n) -> Tree(b) ^ e ! n {
 case tree {
  EmptyTree -> EmptyTree
  Single(x) -> Single(map_node(x, with))
  Deep(m, l, mid, r) -> Deep(m, map_digit(l, with), map_tree(mid, with), map_digit(r, with))
 }
}

main = () {
 t = Deep(4, One(Leaf(1)), Single(Branch2(2, Leaf(2), Leaf(3))), One(Leaf(4)))
 assert(map_tree(t, (x) { x + 1 }) == Deep(4, One(Leaf(2)), Single(Branch2(2, Leaf(3), Leaf(4))), One(Leaf(5))))
}
