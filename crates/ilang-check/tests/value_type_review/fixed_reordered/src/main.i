later=(){put(r,"s")}
Read=r
r=cell(1)
put(A)=(c:Cell(A),x:A){c.set(x)}
cell(A)=(x:A){Cell(get:(){x},set:(y:A){Nil})}
Cell(A) {Cell(get:()->A,set:(A)->Nil)}
