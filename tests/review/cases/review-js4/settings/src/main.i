Await { Await }
Owner { Owner }
FsError { FsError }
Fs { Fs(read: () -> BitArray ^ FsError ! Await | Owner) }
Settings { Settings(read: () -> BitArray ^ FsError ! Await | Owner) }

read_file = (f ! Fs) { f.read() }
filesystem = (f ! Fs) {
  Settings(read: () {
    ! Fs = f
    read_file()
  })
}
fallback = () { Settings(read: () { <<>> }) }
use = (settings ! Settings) { settings.read() }


main = () {
  ! Await = Await
  ! Owner = Owner
  ! Fs = Fs(read: () { <<1, 2>> })
  ! Settings = filesystem()
  ! Fs = Fs(read: () { <<3>> })
  assert(use() == <<1, 2>>)
  ! Settings = fallback()
  assert(use() == <<>>)
}
