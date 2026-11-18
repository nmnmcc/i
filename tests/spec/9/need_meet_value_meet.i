// spec: 4.4, 9.2
// expect: pass

// Two needs of one service are one entry: their meet, as the value meet of
// §4.4. S's parameter goes in the same direction, so S(Never) and S(Dep)
// meet in S(Never) on both paths.
S(N) = (f = () <- N -> ())
Dep = (v = Int)

needs_never = () <- (s : S(Never)) { () }

needs_dep = () <- (s : S(Dep)) { () }

// The need path: the two needs merge to S(Never).
both: () <- (S(Never)) -> () = () {
	needs_never()
	needs_dep()
}

// The value path: the intersection of the two forms is S(Never).
meet = (x : S(Never) & S(Dep)) -> S(Never) { x }

// As a value union the same-direction argument gives the row union Dep.
join = (x : { S(Never) | S(Dep) }) -> S(Dep) { x }

main = () {
	_syn3_0 : S(Never) = S(f = () { () })
	{
		both()
		meet(S(f = () { () }))
		join(S(f = () { () }))
		()
	} <- (S(f = () { () }))
}
