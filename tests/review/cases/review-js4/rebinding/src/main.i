Web = ()
Native = ()
TargetKind = Web | Native
Target = (TargetKind)
read = () <- (t : Target) { t |> (Target(Web)) { 1 } | (Target(Native)) { 2 } }
probe = () <- (t : Target) {
	alias = t
	captured = () { alias |> (Target(Web)) { 1 } | (Target(Native)) { 2 } }
	(captured(), read()) <- (Target(Native))
}

main = () { assert(probe() == (1, 2)) <- (Target(Web)) }
