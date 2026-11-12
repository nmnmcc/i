File = (index = Int, generation = Int)
@host("review:fs/v1")
Fs = (open = () -> File)

main = () <- (fs: Fs) { fs.open() }
