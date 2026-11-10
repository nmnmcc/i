from probe import case
case('fix_location','''Cell(A) {Cell(get:()->A,set:(A)->Nil)}
cell(A)=(x:A){Cell(get:(){x},set:(y:A){Nil})}
put(A)=(c:Cell(A),x:A){c.set(x)}
(a,b)=(cell(1),cell("s"))
First=a
Second=b
later=(){put(b,1)}''','E0501 fixation note at Second=b')
case('closed_annotated_fn','f = (n: Int) -> Int { n+1 }\nF=f\ng: F = (n: Int)->Int {n}\nv=g(1)','pass')
case('no_library_rule','((Schema,make) = review/lib)\nuser = make((s:String)->(String,Int) {(s,1)}, (v:(String,Int))->String {v.0})\nSchema(User,_) = user\nx:User = ("s",1)','pass',{'src/lib.i':'(Schema,make)\nSchema(D,E) {Schema(decode:(String)->D,encode:(E)->String)}\nmake(D,E) = (d:(String)->D,e:(E)->String) {Schema(d,e)}'})
case('foreign_weak','((Codec) = review/lib)\nCodec(Remote,_) = lib.size','E0534 with alias hint',{'src/lib.i':'(Codec,size,Local)\nCodec(D,E) {Codec(decode:(String)->D,encode:(E)->String)}\nmake(D,E)=(d:(String)->D,e:(E)->String){Codec(d,e)}\nsize=make((s:String)->Int {1},(n:Int)->String {""})\nCodec(Local,_) = size'})
case('early_field','n: Int = 1\nN=n\nBox {Box(N)}','E0531')
case('early_external','n: Int = 1\nN=n\n@external("env","f")\nf:(N)->Int','E0531')
case('union_samehead_alias_only','Box(A){Box(A)}\nv = True |> {True -> Box(1) False -> Box(2)}\nWhole=v\nx: Whole=Box(3)','pass')
case('whole_ref_normalize','Box(A){Box(A)}\nv = True |> {True -> Box(1) False -> Box(2)}\nWhole=v\nBox(Item)=v','pass')
# Value-dependent alias chain scaling compared with ordinary alias chain.
for n in [1024,2048]:
 for val in [True,False]:
  src=('v: Int = 1\nA0 = v\n' if val else 'A0 = Int\n')+'\n'.join(f'A{i} = A{i-1}' for i in range(1,n))+'\nx: A'+str(n-1)+' = 2'
  case(('value' if val else 'ordinary')+'chain'+str(n),src,'pass quickly')
