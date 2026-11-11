(choose, Target(..), Fs(..), Js(..), Wasi(..))
Js { Js }
Wasi { Wasi }
Target { JsTarget(Js) NodeJS(Js) WasiTarget(Wasi) Native Host }
Fs { Fs(read_file: (String) -> BitArray ^ String) }
read_file = (path, fs ! Fs) { fs.read_file(path) }
choose = (t ! Target) {
  t |> {
    NodeJS(_) -> read_file("settings.txt")
    Native -> read_file("settings.txt")
    _ -> <<>>
  }
}
