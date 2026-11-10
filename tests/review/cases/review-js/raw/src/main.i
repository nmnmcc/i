@host("wasi_snapshot_preview1")
Wasi { Wasi(fd_read: (Int, Int, Int, Int) -> Int) }
@main
main = (w ! Wasi) { w.fd_read(0, 0, 0, 0) }
