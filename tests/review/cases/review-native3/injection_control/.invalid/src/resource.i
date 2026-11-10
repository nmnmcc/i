// old-syntax-check: E0101
export (Stream, Raw(..), open)
Stream { Stream(Int) }
Raw { Raw(open: () -> Int) }
open = (raw ! Raw) { Stream(raw.open()) }
