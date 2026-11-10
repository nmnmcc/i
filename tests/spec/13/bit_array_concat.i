// spec: 13
// expect: pass
(
	i/core
)

check = (got: BitArray, want: BitArray) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(core.inspect(got) <> " is not " <> core.inspect(want))
	}
}

same = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

@test
whole_bytes = () {
	check(core.bit_array_concat([]), <<>>)
	check(core.bit_array_concat([<<1>>]), <<1>>)
	check(core.bit_array_concat([<<1>>, <<2, 3>>, <<>>, <<4>>]), <<1, 2, 3, 4>>)
}

@test
parts_that_are_not_whole_bytes = () {
	check(
		core.bit_array_concat([<<1:u1>>, <<2:u2>>, <<255>>])
		<<1:u1, 2:u2, 255>>
	)
	check(
		core.bit_array_concat([<<5:u3>>, <<5:u3>>, <<5:u3>>])
		<<5:u3, 5:u3, 5:u3>>
	)
	check(core.bit_array_concat([<<1:u1>>, <<>>, <<1:u7>>]), <<129>>)
	check(core.bit_array_concat([<<1, 2:u4>>, <<3:u4, 4>>]), <<1, 35, 4>>)
	same(
		core.inspect(core.bit_array_concat([<<1:u1>>, <<2:u2>>, <<255>>]))
		"<<223, 7:u3>>"
	)
	same(
		core.inspect(
			core.bit_size(core.bit_array_concat([<<0:u5>>, <<0:u5>>, <<0:u5>>]))
		)
		"15"
	)
}

@test
slices = () {
	<<170, 85, 255>> |> {
		<<_:u3, middle:bits(13), rest:bits>> -> {
			check(
				core.bit_array_concat([middle, middle, rest])
				<<middle:bits, middle:bits, rest:bits>>
			)
			check(
				core.bit_array_concat([rest, middle])
				<<rest:bits, middle:bits>>
			)
		}
		_ -> panic("no")
	}
}
