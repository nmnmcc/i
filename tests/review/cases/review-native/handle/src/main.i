File { File(index: Int, generation: Int) }
@host("review:fs/v1")
Fs { Fs(open: () -> File) }
@main
main = (fs ! Fs) { fs.open() }
