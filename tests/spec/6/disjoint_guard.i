// spec: 6.9, 7.4
// expect: warning W0503

// W520: a false guard falls through; an inequality guard can succeed.
no = (x : 1) { x |> (_ ? x == 2) { "wrong" } | (_) { "fallback" } }
yes = (x : 1) { x |> (_ ? x != 2) { "chosen" } | (_) { "wrong" } }

main = () {
	assert(no(1) == "fallback")
	assert(yes(1) == "chosen")
}
