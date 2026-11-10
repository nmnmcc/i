(i/random, i/fail, std/list, (Option, Some, None) = std/option)
(Gen, Tree(..), make, sample, constant, map, flat_map, int, int_tree, shrink)
Gen(A) { Gen((Int,random.Generator)->Option(Tree(A))) }
Tree(A) { Tree(value:A,children:()->List(Tree(A))) }
make(A) = (f:(Int,random.Generator)->Option(Tree(A))) -> Gen(A) { Gen(f) }
sample(A) = (gen:Gen(A),size:Int,g:random.Generator) -> Option(Tree(A)) {
 Gen(f)=gen
 f({ size<0 } |> { True->0 False->size },g)
}
constant(A) = (a:A) -> Gen(A) { Gen((_,_) { Some(Tree(a,(){ [] })) }) }
map_tree(A,B) = (t:Tree(A),f:(A)->B) -> Tree(B) {
 Tree(f(t.value),(){ list.map(t.children(),(c){map_tree(c,f)}) })
}
map(A,B) = (gen:Gen(A),f:(A)->B) -> Gen(B) {
 Gen((s,g){ sample(gen,s,g) |> { None->None Some(t)->Some(map_tree(t,f)) } })
}
bind_node(A,B) = (outer:Tree(A),inner:Tree(B),f:(A)->Gen(B),s:Int,g:random.Generator) -> Tree(B) {
 Tree(inner.value,(){
   a=list.filter_map(outer.children(),(c){bind_outer(c,f,s,g)})
   b=list.map(inner.children(),(c){bind_node(outer,c,f,s,g)})
   list.append(a,b)
 })
}
bind_outer(A,B) = (outer:Tree(A),f:(A)->Gen(B),s:Int,g:random.Generator) -> Option(Tree(B)) {
 sample(f(outer.value),s,g) |> { None->None Some(inner)->Some(bind_node(outer,inner,f,s,g)) }
}
flat_map(A,B) = (gen:Gen(A),f:(A)->Gen(B)) -> Gen(B) {
 Gen((s,g){
  (left,right)=random.split(g)
  sample(gen,s,left) |> { None->None Some(t)->bind_outer(t,f,s,right) }
 })
}
int_tree = (x:Int,t:Int) -> Tree(Int) {
 Tree(x,(){
   { x==t } |> {
    True->[]
    False->{
     go=(q:Int,acc:List(Int))->List(Int){
       q |> { 0->list.reverse(acc) _->go(q/2,[x-q,..acc]) }
     }
     vals=list.unique([t,..go({x-t}/2,[])])
     list.map(vals,(v){int_tree(v,t)})
    }
   }
 })
}
int = (lo:Int,hi:Int) -> Gen(Int) {
 Gen((_,g){
  fail.catch((){random.int(g,hi-lo+1)}) |> {
   Error(_)->None
   Ok((v,_))->{
    target={lo>0} |> {True->lo False->{hi<0} |> {True->hi False->0}}
    Some(int_tree(v+lo,target))
   }
  }
 })
}
shrink(A) = (t:Tree(A),budget:Int,f:(A)->Bool) -> (A,List(Int),Int) {
 descend(t,budget,[],0,f)
}
descend(A)=(current:Tree(A),left:Int,path:List(Int),count:Int,f:(A)->Bool)->(A,List(Int),Int){
 {left==0} |> {
  True->(current.value,list.reverse(path),count)
  False->scan(current,current.children(),left,path,count,0,f)
 }
}
scan(A)=(current:Tree(A),children:List(Tree(A)),left:Int,path:List(Int),count:Int,index:Int,f:(A)->Bool)->(A,List(Int),Int){
 {left==0} |> {
  True->(current.value,list.reverse(path),count)
  False->children |> {
   []->(current.value,list.reverse(path),count)
   [c,..cs]->f(c.value) |> {
    True->descend(c,left-1,[index,..path],count+1,f)
    False->scan(current,cs,left-1,path,count+1,index+1,f)
   }
  }
 }
}
