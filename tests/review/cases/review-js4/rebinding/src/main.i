Web = ()
Native = ()
Target = Web | Native
read = () <- (t : Target) { t |> (Web) { 1 } | (Native) { 2 } }
probe = () <- (t : Target) {
	alias = t
	captured = () { alias |> (Web) { 1 } | (Native) { 2 } }
	(captured(), read()) <- (Native)
}

main = () { assert(probe() == (1, 2)) <- (Web) }
