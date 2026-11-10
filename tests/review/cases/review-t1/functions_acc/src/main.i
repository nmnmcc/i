(list = std/list)
Box(A) { Box(get: () -> A) }
@main
main = () {
 b = list.fold([1,2,3], Box(() { 0 }), (acc,x) { Box(() { acc.get()+x }) })
 assert(b.get()==6) Nil
}
