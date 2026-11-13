// §4.8 rule 13 and §6.7: `v.f` on a value whose known type is a union of
// custom types, every constructor of which has the field; `v.k` on a union
// of tuples.
Cat = (name = String, lives = Int)

Dog = (name = String, good = Bool)

Rock = (weight = Int)

Num = (value = Int)

Text = (value = String)

Circle = (radius = Float, label = String)
Square = (side = Float, label = String)
Shape = Circle | Square

Box(A) = (value = A)

// Every member has the field: the type is the union of the field types.
name = (p : { Cat | Dog }) { p.name }

value = (v : { Num | Text }) { v.value }

label = (p : { Shape | Cat }) { p |> (Cat(..)) { p.name } | (_) { p.label } }

generic = (b : { Box(Int) | Num }) { b.value }

// The field may be in any position.
Pair = (first = Int, name = String)

position = (p : { Pair | Cat }) { p.name }

// A member without the field, a member that is not a custom type, and a
// type with no constructors.
Opaque = Opaque

missing = (p : { Cat | Rock }) { p.name }

not_custom = (p : { Cat | Int }) { p.name }

opaque = (o : Opaque) { o.name }

no_known_type = (p) { p.name }

// Narrowing through a known constructor (§7.5).
known_ctor = (s : Shape) {
	s |> (Circle(..)) { s.radius } | (Square(..)) { s.side }
}

// Narrowing of the known type of a catch-all (rule 12).
narrowed = (x : { Cat | Rock }) {
	x |> (_ : Rock) { "" } | (other) { other.name }
}

// A value with no written type, whose known type comes from a call.
make = (n : Int) { Cat(name = "tom", lives = n) }

from_call = () { make(9).lives }

from_update = (c : Cat) { Cat(..c, name = "x").lives }

// A local function, generalized: matching gives its result a known type.
local_call = (c : { Cat | Dog }) {
	same = <A>(x : A) -> A { x }
	same(c).name
}

// A call of a function with no known type has none.
apply = (f, c : { Cat | Dog }) { f(c).name }

// The type of `v` must be a subtype of its known type: a function
// expression whose parameter is known from an expected type.
each = <A, B>(xs : List(A), f : (A) -> B) -> List(B) { [] }

checked = (rocks : List(Rock)) { each(rocks, (r) { r.weight }) }

wrong = (rocks : List(Rock)) {
	f : (Cat) -> Int = (c) { c.lives }
	each(rocks, f)
}

// `.k` on a union of tuples, with more than k parts each.
part = (t : { (Int, String) | (Float, Bool, Int) }) { t.1 }

short = (t : { (Int) | (Int, Int) }) { t.1 }

not_tuple = (t : { (Int, Int) | Nil }) { t.0 }
