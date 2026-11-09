export (Hid(..), Opaque, Ex, ExList, Box(..), dec, pub, via_alias, via_hidden_alias, named, b, c, inferred)

Hidden { Hidden }
Handle {}

// The fields of a type exported with `(..)` are shown.
Hid { Hid(h: Hidden) }
// A type exported alone does not show its fields.
Opaque { Opaque(h: Hidden) }
Ex = Hidden
ExList = List((Int, Hidden))
Box(a) { Box(a, Int) }

@external("a", "b")
dec: (Hidden) -> Int

pub: (Hidden) -> Int = (h: Hidden) { 1 }

// An alias that is not exported is looked through.
Priv = Int
via_alias: Priv = 1
PrivHidden = List(Hidden)
via_hidden_alias: PrivHidden = []

named = inner: Hidden = Hidden

// Only the exported names count.
(a: Hidden, b: Int) = (Hidden, 1)
(c: Hidden, d: Int) = (Hidden, 1)
private: Hidden = Hidden

// TODO(M3): an inferred type is checked by type inference.
inferred = Hidden
