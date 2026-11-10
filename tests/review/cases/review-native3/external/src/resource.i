export (Stream, Raw, open, seed)
Stream { Stream(Int) }
Raw { Raw(open: () -> Int) }
open = (raw ! Raw) { Stream(raw.open()) }
seed = () { Raw(() { 123 }) }
