import (probe/stream)
Bias { Bias(value: Int) }
source = () -> stream.Stream(Int, Never, Bias) {
 stream.make((b ! Bias) { stream.Next(b.value, stream.new()) })
}
@main
main = () {
 ! Bias = Bias(10)
 assert(stream.to_list(source()) == [10])
}
