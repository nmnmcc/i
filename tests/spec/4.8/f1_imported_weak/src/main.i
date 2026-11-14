(lib = probe/lib, i/stdio, core = i/core)
(main)
choose = (x: String) { "wrong" } | (x: { Int | String }) { 1 }
main = () {
    _ = lib.weak(0)
    n: Int = choose(lib.weak("s"))
    stdio.print(core.inspect(n + 1))
}
