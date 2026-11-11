(i/core, i/request)
(work, Pause)
@host("review:pause", "async")
Pause { Pause(next: () -> Float) }
work = (x: Float, pause ! Pause) -> Float ! Pause | request.Await | request.Owner {
 y = x *. 0.5
 z = pause.next()
 // Both x and y must survive the host handoff; return exact zero sign.
 { core.float_to_bits(x) == core.float_to_bits(y) } |> {
  True -> x
  False -> y +. z
 }
}
