@host("wasi_snapshot_preview1")
Wasi = (fd_read = (Int, Int, Int, Int) -> Int)

main = () <- (w : Wasi) { w.fd_read(0, 0, 0, 0) }
