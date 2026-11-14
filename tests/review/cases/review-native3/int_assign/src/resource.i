export(Stream, Raw, open, seed)
Stream = (Int)
Raw = (open = () -> Int)
open = () <- (raw : Raw) { Stream(raw.open()) }
seed = () { Raw(open = () { 123 }) }
