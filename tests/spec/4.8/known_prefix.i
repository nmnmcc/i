// spec: 4.8
// expect: pass

// §5.4: slicing a prefix gives whole String, not the original value set.
f = (s: String) { "a" <> r = s  r == "bc" }
@test
prefixes = () {
	s = "abc"
	assert(s |> { "a" <> r -> r == "bc"  _ -> False })
	assert(f(s))
}
