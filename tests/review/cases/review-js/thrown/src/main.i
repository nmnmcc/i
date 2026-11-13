Value = Opaque
Thrown = (Value)
@host("probe:js/v1")
Js = (call = (Value) <- (Fail(Thrown)) -> Value)

main = () { Nil }
