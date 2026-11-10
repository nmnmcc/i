from probe import case
# Equivalence and error location probes.
for name,source,expected in [
 ('same_type_bounds','Box(A) {Box(A)}\nv = True |> {True -> Box(1) False -> Box(2)}\nBox(Item) = v','pass'),
 ('union_annotated','Box(A) {Box(A)}\nv: Box(Int | String) = True |> {True -> Box(1) False -> Box("s")}\nBox(Item) = v','pass'),
 ('alias_constant_nested','Pair(A) = (A,Int)\nAlias = Int\nBox(A) {Box(A)}\nv = (Box(1),1)\nPair(Box(Item)) = v','pass'),
 ('alias_repeated_nested','Duo(A) = (A,A)\nWrap(A) = Duo(A)\nv = (1,1)\nWrap(Item) = v','E0532'),
 ('alias_no_binder','Unit = (Int,Int)\nBox(A) {Box(A)}\nv=Box((1,2))\nBox(Unit(_))=v','arity error'),
 ('prior_weak','id=(x){x}\na=id(1)\nb=id(a)\nB=b','E0534'),
 ('earlier_constraints','Box(A) {Box(A)}\nid=(x){x}\na=id(1)\nf=(x){(a,x)}\nb=f(2)\nB=b','E0534'),
 ('invariant_general','Cell(A) {Cell(run:(A)->A)}\nmake=(){Cell((x){x})}\nSame=make','E0533'),
 ('batch_recovery','Box(A,B) {Box(A,B)}\nv=Box(1,(x){x})\nBox(Good,_) = v\nBox(_,Bad) = v\nerror: Int = "s"','E0533 plus independent E0501'),
 ('local_alias_escape','id=(x){x}\nv=id(id)\nf=(){copy=v\ng:copy=(x:Int){x}\ng}\nG=f','reject'),
]:case(name,source,expected)
# Top-level reordering of a weak fixed source and future widening.
lines=['Cell(A) {Cell(get:()->A,set:(A)->Nil)}','cell(A)=(x:A){Cell(get:(){x},set:(y:A){Nil})}','put(A)=(c:Cell(A),x:A){c.set(x)}','r=cell(1)','Read=r','later=(){put(r,"s")}']
case('fixed_later', '\n'.join(lines),'reject with fixation note')
case('fixed_reordered', '\n'.join(reversed(lines)),'same reject with note')
# Long shallow chains do not expand to a large type.
for n in [64,128,256,512]:
 src='v: Int = 1\nA0 = v\n'+'\n'.join(f'A{i} = A{i-1}' for i in range(1,n))+'\nx: A'+str(n-1)+' = 2'
 case('chain'+str(n),src,'pass quickly')
