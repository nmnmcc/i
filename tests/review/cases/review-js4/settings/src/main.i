Await = ()
Owner = ()
FsError = ()
Fs = (read = () <- (Fail(FsError), Await, Owner) -> BitArray)
Settings = (read = () <- (Fail(FsError), Await, Owner) -> BitArray)

read_file = () <- (f: Fs) { f.read() }
filesystem = () <- (f: Fs) { Settings(read = () { read_file() <- (f) }) }
fallback = () { Settings(read = () { <<>> }) }
use = () <- (settings: Settings) { settings.read() }

main = () {
	{
		assert(use() == <<1, 2>>)
		assert(use() == <<>>) <- (fallback())
	} <- (Fs(read = () { <<3>> })) <- (filesystem()) <- (
		Await,
		Owner,
		Fs(read = () { <<1, 2>> }),
	)
}
