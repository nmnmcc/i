(i/control)
g = (b, x: Float) -> Float {
 y = control.suspend(b, (k) { control.resume(k, 1.0) })
 x *. x +. y
}
f = (b, x: Float) -> Float {
 y = g(b, x)
 y +. 0.25
}
h = (b, x: Float) -> Float {
 y = f(b, x)
 y +. 0.5
}
@main
main = () {
 control.boundary((b) { h(b, 1.0e308) })
 Nil
}
