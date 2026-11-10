import (probe/stream)
Bias { Bias(value: Int) }
end_bias = () -> stream.Stream(Int, Never, Bias) {
 stream.make(() -> stream.Step(Int, Never, Bias) ! Bias { stream.Done })
}
source = () -> stream.Stream(Int, Never, Bias) {
 stream.make((b ! Bias) { stream.Next(b.value, end_bias()) })
}
@main
main = () {
 s = source()
 {
  ! Bias = Bias(10)
  assert(stream.to_list(s) == [10])
 }
 {
  ! Bias = Bias(20)
  assert(stream.to_list(s) == [20])
 }
 adapted = stream.map_environment(s, (body: () -> stream.Step(Int, Never, Bias) ! Bias) {
  ! Bias = Bias(30)
  body()
 })
 assert(stream.to_list(adapted) == [30])
}
