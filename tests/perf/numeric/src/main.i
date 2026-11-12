((Io) = i/io, i/core, numeric/kernels)
main = (io ! Io) {
  io.print(core.inspect(kernels.horner(0.5)))
  io.print(core.inspect(kernels.integer(0.5)))
}
