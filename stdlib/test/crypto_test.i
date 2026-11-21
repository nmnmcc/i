(
	testlib = (test) = i/test
	i/core
	i/process
	i/request
	i/fail
	i/crypto
	i/bit_array
	i/string
)

hex = (x : BitArray) -> String { string.to_lower(bit_array.base16_encode(x)) }

unhex = (s : String) -> BitArray {
	fail.catch(() { bit_array.base16_decode(s) }) |>
		(Ok(b)) { b }
		& (Error(_)) { panic("test hex") }
}

test("sha256_0", () {
	assert(
		hex(
			crypto.sha256(unhex("")),
		) ==
			"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
	)
	()
})

test("sha256_1", () {
	assert(
		hex(
			crypto.sha256(unhex("616263")),
		) ==
			"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
	)
	()
})

test("sha256_2", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"6162636462636465636465666465666765666768666768696768696a68696a6b696a6b6c6a6b6c6d6b6c6d6e6c6d6e6f6d6e6f706e6f7071",
				),
			),
		) ==
			"248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
	)
	()
})

test("sha256_3", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"61626364656667686263646566676869636465666768696a6465666768696a6b65666768696a6b6c666768696a6b6c6d6768696a6b6c6d6e68696a6b6c6d6e6f696a6b6c6d6e6f706a6b6c6d6e6f70716b6c6d6e6f7071726c6d6e6f707172736d6e6f70717273746e6f707172737475",
				),
			),
		) ==
			"cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
	)
	()
})

test("sha256_4", () {
	assert(
		hex(
			crypto.sha256(unhex("00")),
		) ==
			"6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d",
	)
	()
})

test("sha256_5", () {
	assert(
		hex(
			crypto.sha256(unhex("00010203040506")),
		) ==
			"57355ac3303c148f11aef7cb179456b9232cde33a818dfda2c2fcb9325749a6b",
	)
	()
})

test("sha256_6", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f30313233343536",
				),
			),
		) ==
			"463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59",
	)
	()
})

test("sha256_7", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f3031323334353637",
				),
			),
		) ==
			"da2ae4d6b36748f2a318f23e7ab1dfdf45acdc9d049bd80e59de82a60895f562",
	)
	()
})

test("sha256_8", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e",
				),
			),
		) ==
			"29af2686fd53374a36b0846694cc342177e428d1647515f078784d69cdb9e488",
	)
	()
})

test("sha256_9", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f",
				),
			),
		) ==
			"fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108",
	)
	()
})

test("sha256_10", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f40",
				),
			),
		) ==
			"4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781",
	)
	()
})

test("sha256_11", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e",
				),
			),
		) ==
			"60780e9451bdc43cf4530ffc95cbb0c4eb24dae2c39f55f334d679e076c08065",
	)
	()
})

test("sha256_12", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f",
				),
			),
		) ==
			"09373f127d34e61dbbaa8bc4499c87074f2ddb10e1b465f506d7d70a15011979",
	)
	()
})

test("sha256_13", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e",
				),
			),
		) ==
			"92ca0fa6651ee2f97b884b7246a562fa71250fedefe5ebf270d31c546bfea976",
	)
	()
})

test("sha256_14", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f",
				),
			),
		) ==
			"471fb943aa23c511f6f72f8d1652d9c880cfa392ad80503120547703e56a2be5",
	)
	()
})

test("sha256_15", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f80",
				),
			),
		) ==
			"5099c6a56203f9687f7d33f4bfdf576d31dc91f6b695ecea38b2770c87631135",
	)
	()
})

test("sha256_16", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfe",
				),
			),
		) ==
			"3f8591112c6bbe5c963965954e293108b7208ed2af893e500d859368c654eabe",
	)
	()
})

test("sha256_17", () {
	assert(
		hex(
			crypto.sha256(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff",
				),
			),
		) ==
			"40aff2e9d2d8922e47afd4648e6967497158785fbd1da870e7110266bf944880",
	)
	()
})

test("sha256_million_a", () {
	message = string.repeat("a", 1000000)
	assert(
		hex(crypto.sha256(<<message:utf8>>)) ==
			"cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
	)
	()
})

test("sha512_0", () {
	assert(
		hex(
			crypto.sha512(unhex("")),
		) ==
			"cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
	)
	()
})

test("sha512_1", () {
	assert(
		hex(
			crypto.sha512(unhex("616263")),
		) ==
			"ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
	)
	()
})

test("sha512_2", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"6162636462636465636465666465666765666768666768696768696a68696a6b696a6b6c6a6b6c6d6b6c6d6e6c6d6e6f6d6e6f706e6f7071",
				),
			),
		) ==
			"204a8fc6dda82f0a0ced7beb8e08a41657c16ef468b228a8279be331a703c33596fd15c13b1b07f9aa1d3bea57789ca031ad85c7a71dd70354ec631238ca3445",
	)
	()
})

test("sha512_3", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"61626364656667686263646566676869636465666768696a6465666768696a6b65666768696a6b6c666768696a6b6c6d6768696a6b6c6d6e68696a6b6c6d6e6f696a6b6c6d6e6f706a6b6c6d6e6f70716b6c6d6e6f7071726c6d6e6f707172736d6e6f70717273746e6f707172737475",
				),
			),
		) ==
			"8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909",
	)
	()
})

test("sha512_4", () {
	assert(
		hex(
			crypto.sha512(unhex("00")),
		) ==
			"b8244d028981d693af7b456af8efa4cad63d282e19ff14942c246e50d9351d22704a802a71c3580b6370de4ceb293c324a8423342557d4e5c38438f0e36910ee",
	)
	()
})

test("sha512_5", () {
	assert(
		hex(
			crypto.sha512(unhex("00010203040506")),
		) ==
			"b7c0b47f42f7202bf7d28d6834bee365fc01ce3f0c8c8df24b4d940406c2e9c230ba88854e946ebcd786c18c748969fdf012362b7c96400604b6058950feaad4",
	)
	()
})

test("sha512_6", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f30313233343536",
				),
			),
		) ==
			"6856647f269c2ee3d8128f0b25427659d880641ef343300dd3cd4679168f58d6527fda70b4ebc854e2065e172b7d58c1536992c0810599259ba84a2b40c65414",
	)
	()
})

test("sha512_7", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f3031323334353637",
				),
			),
		) ==
			"8b12b2f6fe400a51d29656e2b8c42a1bbfe6fcf3e425da430db05d1a2dda14790dee20fa8b22d8762afffe4988a5c98a4430d22a17e41e23d90fa61ab75671a9",
	)
	()
})

test("sha512_8", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e",
				),
			),
		) ==
			"9dc9c5598e55dc42955695320839788e353f1d7f6ba74df74c80a8a52f463c0697f57f68835d1418f4ce9b6530cd79bd0f4c6f7e13c93feb1218c0b65c2c0561",
	)
	()
})

test("sha512_9", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f",
				),
			),
		) ==
			"ee4320ebaf3fdb4f2c832b137200c08e235e0fa7bbd0eb1740c7063ba8a0d151da77e003398e1714a955d475b05e3e950b639503b452ec185de4229bc4873949",
	)
	()
})

test("sha512_10", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f40",
				),
			),
		) ==
			"02856cef735f9acec6b9e33f0fbc8f9804d2aa54187f382b8ae842e5d3696c07459aad2a5aed25ea5e117eb1c7ba35da6a7a8adce9e6afe3ad79e9fa42d5bba8",
	)
	()
})

test("sha512_11", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e",
				),
			),
		) ==
			"a1a111449b198d9b1f538bad7f3fc1022b3a5b1a5e90a0bc860de8512746cbc31599e6c834de3a3235327af0b51ff57bf7acf1974a73014d9c3953812edc7c8d",
	)
	()
})

test("sha512_12", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f",
				),
			),
		) ==
			"c5fbd731d19d2ae1180f001be72c2c1aaba1d7b094b3748880e24593b8e117a750e11c1bd867cc2f96dace8c8b74abd2d5c4f236be444e77d30d1916174070b9",
	)
	()
})

test("sha512_13", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e",
				),
			),
		) ==
			"eab89674feaa34e27aebeeff3c0a4d70070bb872d5e9f186cf1dbbdee517b6e35724d629ff025a5b07185e911ada7e3c8acf830aa0e4f71777bd2d44f504f7f0",
	)
	()
})

test("sha512_14", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f",
				),
			),
		) ==
			"1dffd5e3adb71d45d2245939665521ae001a317a03720a45732ba1900ca3b8351fc5c9b4ca513eba6f80bc7b1d1fdad4abd13491cb824d61b08d8c0e1561b3f7",
	)
	()
})

test("sha512_15", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f80",
				),
			),
		) ==
			"1d9da57fbbdab09afb3506ab2d223d06109d65c1c8ad197f50138f714bc4c3f2fe5787922639c680acad1c651f955990425954ce2cba0c5cc83f2667d878eb0f",
	)
	()
})

test("sha512_16", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfe",
				),
			),
		) ==
			"15025c9d135861ff5a549df0bfd6c398fd126613496d4e97627651e68b7b1f80407f187d7978464f0f78bfeea787600faaebbe991eddb60671cd0ce874f0a744",
	)
	()
})

test("sha512_17", () {
	assert(
		hex(
			crypto.sha512(
				unhex(
					"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff",
				),
			),
		) ==
			"1e7b80bc8edc552c8feeb2780e111477e5bc70465fac1a77b29b35980c3f0ce4a036a6c9462036824bd56801e62af7e9feba5c22ed8a5af877bf7de117dcac6d",
	)
	()
})

test("sha512_million_a", () {
	message = string.repeat("a", 1000000)
	assert(
		hex(crypto.sha512(<<message:utf8>>)) ==
			"e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973ebde0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b",
	)
	()
})

test("hmac_sha256_rfc4231_1", () {
	assert(
		{
			e4_call_arg_0 = unhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
			e4_call_arg_1 = unhex("4869205468657265")
			fail.catch(() { crypto.hmac_sha256(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
				),
			),
	)
	()
})

test("hmac_sha256_rfc4231_2", () {
	assert(
		{
			e4_call_arg_0 = unhex("4a656665")
			e4_call_arg_1 = unhex(
				"7768617420646f2079612077616e7420666f72206e6f7468696e673f",
			)
			fail.catch(() { crypto.hmac_sha256(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
				),
			),
	)
	()
})

test("hmac_sha256_rfc4231_3", () {
	assert(
		{
			e4_call_arg_0 = unhex("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
			e4_call_arg_1 = unhex(
				"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
			)
			fail.catch(() { crypto.hmac_sha256(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
				),
			),
	)
	()
})

test("hmac_sha256_rfc4231_4", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"0102030405060708090a0b0c0d0e0f10111213141516171819",
			)
			e4_call_arg_1 = unhex(
				"cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
			)
			fail.catch(() { crypto.hmac_sha256(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b",
				),
			),
	)
	()
})

test("hmac_sha256_rfc4231_5", () {
	assert(
		{
			e4_call_arg_0 = unhex("0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c")
			e4_call_arg_1 = unhex("546573742057697468205472756e636174696f6e")
			fail.catch(() { crypto.hmac_sha256(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"a3b6167473100ee06e0c796c2955552bfa6f7c0a6a8aef8b93f860aab0cd20c5",
				),
			),
	)
	()
})

test("hmac_sha256_rfc4231_6", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
			)
			e4_call_arg_1 = unhex(
				"54657374205573696e67204c6172676572205468616e20426c6f636b2d53697a65204b6579202d2048617368204b6579204669727374",
			)
			fail.catch(() { crypto.hmac_sha256(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
				),
			),
	)
	()
})

test("hmac_sha256_rfc4231_7", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
			)
			e4_call_arg_1 = unhex(
				"5468697320697320612074657374207573696e672061206c6172676572207468616e20626c6f636b2d73697a65206b657920616e642061206c6172676572207468616e20626c6f636b2d73697a6520646174612e20546865206b6579206e6565647320746f20626520686173686564206265666f7265206265696e6720757365642062792074686520484d414320616c676f726974686d2e",
			)
			fail.catch(() { crypto.hmac_sha256(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
				),
			),
	)
	()
})

test("hmac_sha512_rfc4231_1", () {
	assert(
		{
			e4_call_arg_0 = unhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
			e4_call_arg_1 = unhex("4869205468657265")
			fail.catch(() { crypto.hmac_sha512(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854",
				),
			),
	)
	()
})

test("hmac_sha512_rfc4231_2", () {
	assert(
		{
			e4_call_arg_0 = unhex("4a656665")
			e4_call_arg_1 = unhex(
				"7768617420646f2079612077616e7420666f72206e6f7468696e673f",
			)
			fail.catch(() { crypto.hmac_sha512(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737",
				),
			),
	)
	()
})

test("hmac_sha512_rfc4231_3", () {
	assert(
		{
			e4_call_arg_0 = unhex("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
			e4_call_arg_1 = unhex(
				"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
			)
			fail.catch(() { crypto.hmac_sha512(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"fa73b0089d56a284efb0f0756c890be9b1b5dbdd8ee81a3655f83e33b2279d39bf3e848279a722c806b485a47e67c807b946a337bee8942674278859e13292fb",
				),
			),
	)
	()
})

test("hmac_sha512_rfc4231_4", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"0102030405060708090a0b0c0d0e0f10111213141516171819",
			)
			e4_call_arg_1 = unhex(
				"cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
			)
			fail.catch(() { crypto.hmac_sha512(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"b0ba465637458c6990e5a8c5f61d4af7e576d97ff94b872de76f8050361ee3dba91ca5c11aa25eb4d679275cc5788063a5f19741120c4f2de2adebeb10a298dd",
				),
			),
	)
	()
})

test("hmac_sha512_rfc4231_5", () {
	assert(
		{
			e4_call_arg_0 = unhex("0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c")
			e4_call_arg_1 = unhex("546573742057697468205472756e636174696f6e")
			fail.catch(() { crypto.hmac_sha512(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"415fad6271580a531d4179bc891d87a650188707922a4fbb36663a1eb16da008711c5b50ddd0fc235084eb9d3364a1454fb2ef67cd1d29fe6773068ea266e96b",
				),
			),
	)
	()
})

test("hmac_sha512_rfc4231_6", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
			)
			e4_call_arg_1 = unhex(
				"54657374205573696e67204c6172676572205468616e20426c6f636b2d53697a65204b6579202d2048617368204b6579204669727374",
			)
			fail.catch(() { crypto.hmac_sha512(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598",
				),
			),
	)
	()
})

test("hmac_sha512_rfc4231_7", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
			)
			e4_call_arg_1 = unhex(
				"5468697320697320612074657374207573696e672061206c6172676572207468616e20626c6f636b2d73697a65206b657920616e642061206c6172676572207468616e20626c6f636b2d73697a6520646174612e20546865206b6579206e6565647320746f20626520686173686564206265666f7265206265696e6720757365642062792074686520484d414320616c676f726974686d2e",
			)
			fail.catch(() { crypto.hmac_sha512(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(
				unhex(
					"e37b6a775dc87dbaa4dfa9f96e5e3ffddebd71f8867289865df5a32d20cdc944b6022cac3c4982b10d5eeb55c3e4de15134676fb6de0446065c97440fa8c6a58",
				),
			),
	)
	()
})

test("hkdf_sha256_rfc5869_1", () {
	assert(
		{
			e4_call_arg_0 = unhex("000102030405060708090a0b0c")
			e4_call_arg_1 = unhex(
				"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
			)
			fail.catch(() {
				crypto.hkdf_extract_sha256(e4_call_arg_0, e4_call_arg_1)
			})
		} ==
			Ok(
				unhex(
					"077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5",
				),
			),
	)
	assert(
		{
			e4_call_arg_0 = unhex(
				"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
			)
			e4_call_arg_1 = unhex("000102030405060708090a0b0c")
			e4_call_arg_2 = unhex("f0f1f2f3f4f5f6f7f8f9")
			e4_call_arg_3 = 42
			fail.catch(() {
				crypto.hkdf_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865",
				),
			),
	)
	()
})

test("hkdf_sha256_rfc5869_2", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf",
			)
			e4_call_arg_1 = unhex(
				"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f",
			)
			fail.catch(() {
				crypto.hkdf_extract_sha256(e4_call_arg_0, e4_call_arg_1)
			})
		} ==
			Ok(
				unhex(
					"06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244",
				),
			),
	)
	assert(
		{
			e4_call_arg_0 = unhex(
				"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f",
			)
			e4_call_arg_1 = unhex(
				"606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf",
			)
			e4_call_arg_2 = unhex(
				"b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff",
			)
			e4_call_arg_3 = 82
			fail.catch(() {
				crypto.hkdf_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87",
				),
			),
	)
	()
})

test("hkdf_sha256_rfc5869_3", () {
	assert(
		{
			e4_call_arg_0 = unhex("")
			e4_call_arg_1 = unhex(
				"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
			)
			fail.catch(() {
				crypto.hkdf_extract_sha256(e4_call_arg_0, e4_call_arg_1)
			})
		} ==
			Ok(
				unhex(
					"19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04",
				),
			),
	)
	assert(
		{
			e4_call_arg_0 = unhex(
				"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
			)
			e4_call_arg_1 = unhex("")
			e4_call_arg_2 = unhex("")
			e4_call_arg_3 = 42
			fail.catch(() {
				crypto.hkdf_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8",
				),
			),
	)
	()
})

test("hkdf_sha512_rfc5869_1", () {
	assert(
		{
			e4_call_arg_0 = unhex("000102030405060708090a0b0c")
			e4_call_arg_1 = unhex(
				"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
			)
			fail.catch(() {
				crypto.hkdf_extract_sha512(e4_call_arg_0, e4_call_arg_1)
			})
		} ==
			Ok(
				unhex(
					"665799823737ded04a88e47e54a5890bb2c3d247c7a4254a8e61350723590a26c36238127d8661b88cf80ef802d57e2f7cebcf1e00e083848be19929c61b4237",
				),
			),
	)
	assert(
		{
			e4_call_arg_0 = unhex(
				"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
			)
			e4_call_arg_1 = unhex("000102030405060708090a0b0c")
			e4_call_arg_2 = unhex("f0f1f2f3f4f5f6f7f8f9")
			e4_call_arg_3 = 42
			fail.catch(() {
				crypto.hkdf_sha512(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"832390086cda71fb47625bb5ceb168e4c8e26a1a16ed34d9fc7fe92c1481579338da362cb8d9f925d7cb",
				),
			),
	)
	()
})

test("hkdf_sha512_rfc5869_2", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf",
			)
			e4_call_arg_1 = unhex(
				"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f",
			)
			fail.catch(() {
				crypto.hkdf_extract_sha512(e4_call_arg_0, e4_call_arg_1)
			})
		} ==
			Ok(
				unhex(
					"35672542907d4e142c00e84499e74e1de08be86535f924e022804ad775dde27ec86cd1e5b7d178c74489bdbeb30712beb82d4f97416c5a94ea81ebdf3e629e4a",
				),
			),
	)
	assert(
		{
			e4_call_arg_0 = unhex(
				"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f",
			)
			e4_call_arg_1 = unhex(
				"606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf",
			)
			e4_call_arg_2 = unhex(
				"b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff",
			)
			e4_call_arg_3 = 82
			fail.catch(() {
				crypto.hkdf_sha512(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"ce6c97192805b346e6161e821ed165673b84f400a2b514b2fe23d84cd189ddf1b695b48cbd1c8388441137b3ce28f16aa64ba33ba466b24df6cfcb021ecff235f6a2056ce3af1de44d572097a8505d9e7a93",
				),
			),
	)
	()
})

test("hkdf_sha512_rfc5869_3", () {
	assert(
		{
			e4_call_arg_0 = unhex("")
			e4_call_arg_1 = unhex(
				"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
			)
			fail.catch(() {
				crypto.hkdf_extract_sha512(e4_call_arg_0, e4_call_arg_1)
			})
		} ==
			Ok(
				unhex(
					"fd200c4987ac491313bd4a2a13287121247239e11c9ef82802044b66ef357e5b194498d0682611382348572a7b1611de54764094286320578a863f36562b0df6",
				),
			),
	)
	assert(
		{
			e4_call_arg_0 = unhex(
				"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
			)
			e4_call_arg_1 = unhex("")
			e4_call_arg_2 = unhex("")
			e4_call_arg_3 = 42
			fail.catch(() {
				crypto.hkdf_sha512(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"f5fa02b18298a72a8c23898a8703472c6eb179dc204c03425c970e3b164bf90fff22d04836d0e2343bac",
				),
			),
	)
	()
})

test("pbkdf2_rfc6070_sha256_1", () {
	assert(
		{
			e4_call_arg_0 = unhex("70617373776f7264")
			e4_call_arg_1 = unhex("73616c74")
			e4_call_arg_2 = 1
			e4_call_arg_3 = 32
			fail.catch(() {
				crypto.pbkdf2_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b",
				),
			),
	)
	()
})

test("pbkdf2_rfc6070_sha256_2", () {
	assert(
		{
			e4_call_arg_0 = unhex("70617373776f7264")
			e4_call_arg_1 = unhex("73616c74")
			e4_call_arg_2 = 2
			e4_call_arg_3 = 32
			fail.catch(() {
				crypto.pbkdf2_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43",
				),
			),
	)
	()
})

test("pbkdf2_rfc6070_sha256_3", () {
	assert(
		{
			e4_call_arg_0 = unhex("70617373776f7264")
			e4_call_arg_1 = unhex("73616c74")
			e4_call_arg_2 = 4096
			e4_call_arg_3 = 32
			fail.catch(() {
				crypto.pbkdf2_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a",
				),
			),
	)
	()
})

test("pbkdf2_rfc6070_sha256_4", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"70617373776f726450415353574f524470617373776f7264",
			)
			e4_call_arg_1 = unhex(
				"73616c7453414c5473616c7453414c5473616c7453414c5473616c7453414c5473616c74",
			)
			e4_call_arg_2 = 4096
			e4_call_arg_3 = 40
			fail.catch(() {
				crypto.pbkdf2_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"348c89dbcbd32b2f32d814b8116e84cf2b17347ebc1800181c4e2a1fb8dd53e1c635518c7dac47e9",
				),
			),
	)
	()
})

test("pbkdf2_rfc6070_sha256_5", () {
	assert(
		{
			e4_call_arg_0 = unhex("7061737300776f7264")
			e4_call_arg_1 = unhex("7361006c74")
			e4_call_arg_2 = 4096
			e4_call_arg_3 = 16
			fail.catch(() {
				crypto.pbkdf2_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(unhex("89b69d0516f829893c696226650a8687")),
	)
	()
})

test("pbkdf2_rfc6070_sha256_6", () {
	assert(
		{
			e4_call_arg_0 = unhex("")
			e4_call_arg_1 = unhex("")
			e4_call_arg_2 = 1
			e4_call_arg_3 = 64
			fail.catch(() {
				crypto.pbkdf2_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"f7ce0b653d2d72a4108cf5abe912ffdd777616dbbb27a70e8204f3ae2d0f6fad89f68f4811d1e87bcc3bd7400a9ffd29094f0184639574f39ae5a1315217bcd7",
				),
			),
	)
	()
})

test("pbkdf2_rfc6070_sha256_7", () {
	assert(
		{
			e4_call_arg_0 = unhex("70")
			e4_call_arg_1 = unhex("73")
			e4_call_arg_2 = 3
			e4_call_arg_3 = 0
			fail.catch(() {
				crypto.pbkdf2_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(unhex("")),
	)
	()
})

test("pbkdf2_rfc6070_sha256_8", () {
	assert(
		{
			e4_call_arg_0 = unhex(
				"6c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e676c6f6e67",
			)
			e4_call_arg_1 = unhex("73616c74")
			e4_call_arg_2 = 2
			e4_call_arg_3 = 65
			fail.catch(() {
				crypto.pbkdf2_sha256(
					e4_call_arg_0,
					e4_call_arg_1,
					e4_call_arg_2,
					e4_call_arg_3,
				)
			})
		} ==
			Ok(
				unhex(
					"3f601773f7aa41d7d91d967c1044a21abae6e73c001e622934d50675fa9d9df4a22f1aafcb866109cac56f00213fc4cebafe1e511a3d21fabb2e15c43f4de5cca6",
				),
			),
	)
	()
})

test("equal_empty", () {
	assert(crypto.constant_time_equal(<<>>, <<>>) == True())
	()
})

test("equal_bytes", () {
	assert(crypto.constant_time_equal(<<1, 2, 255>>, <<1, 2, 255>>) == True())
	()
})

test("equal_first", () {
	assert(crypto.constant_time_equal(<<0, 2>>, <<1, 2>>) == False())
	()
})

test("equal_last", () {
	assert(crypto.constant_time_equal(<<1, 2>>, <<1, 3>>) == False())
	()
})

test("equal_length", () {
	assert(crypto.constant_time_equal(<<1>>, <<1, 0>>) == False())
	()
})

test("equal_bits", () {
	assert(crypto.constant_time_equal(<<5:u3>>, <<5:u3>>) == True())
	()
})

test("unequal_bits", () {
	assert(crypto.constant_time_equal(<<4:u3>>, <<5:u3>>) == False())
	()
})

test("unequal_bitlength", () {
	assert(crypto.constant_time_equal(<<0:u3>>, <<0:u4>>) == False())
	()
})

test("hmac_alignment", () {
	assert(
		fail.catch(() { crypto.hmac_sha256(<<1:u1>>, <<>>) }) ==
			Error(crypto.UnalignedInput()),
	)
	()
})

test("hmac512_alignment", () {
	assert(
		fail.catch(() { crypto.hmac_sha512(<<>>, <<1:u1>>) }) ==
			Error(crypto.UnalignedInput()),
	)
	()
})

test("hkdf_negative", () {
	assert(
		fail.catch(() { crypto.hkdf_sha256(<<>>, <<>>, <<>>, -1) }) ==
			Error(crypto.InvalidLength()),
	)
	()
})

test("hkdf_limit", () {
	assert(
		fail.catch(() { crypto.hkdf_sha256(<<>>, <<>>, <<>>, 8161) }) ==
			Error(crypto.InvalidLength()),
	)
	()
})

test("hkdf512_limit", () {
	assert(
		fail.catch(() { crypto.hkdf_sha512(<<>>, <<>>, <<>>, 16321) }) ==
			Error(crypto.InvalidLength()),
	)
	()
})

test("hkdf_short_prk", () {
	assert(
		fail.catch(() { crypto.hkdf_expand_sha256(<<1>>, <<>>, 1) }) ==
			Error(crypto.InvalidLength()),
	)
	()
})

test("pbkdf_zero_iterations", () {
	assert(
		fail.catch(() { crypto.pbkdf2_sha256(<<>>, <<>>, 0, 1) }) ==
			Error(crypto.InvalidIterations()),
	)
	()
})

test("pbkdf_negative_iterations", () {
	assert(
		fail.catch(() { crypto.pbkdf2_sha256(<<>>, <<>>, -1, 1) }) ==
			Error(crypto.InvalidIterations()),
	)
	()
})

test("pbkdf_negative_length", () {
	assert(
		fail.catch(() { crypto.pbkdf2_sha256(<<>>, <<>>, 1, -1) }) ==
			Error(crypto.InvalidLength()),
	)
	()
})

test("pbkdf_limit", () {
	assert(
		fail.catch(() { crypto.pbkdf2_sha256(<<>>, <<>>, 1, 137438953441) }) ==
			Error(crypto.InvalidLength()),
	)
	()
})

test("pbkdf_alignment", () {
	assert(
		fail.catch(() { crypto.pbkdf2_sha256(<<1:u1>>, <<>>, 1, 1) }) ==
			Error(crypto.UnalignedInput()),
	)
	()
})

test("random_bytes_fake", () {
	{
		assert(fail.catch(() { crypto.random_bytes(3) }) == Ok(<<0, 0, 7>>))
		assert(fail.catch(() { crypto.random_bytes(0) }) == Ok(<<>>))
		()
	} <- (process.Entropy(entropy = (n) { Ok(<<7:u(n * 8)>>) }))
})

test("random_bytes_negative", () {
	{
		assert(
			fail.catch(() { crypto.random_bytes(-1) }) ==
				Error(crypto.InvalidLength()),
		)
		()
	} <- (
		process.Entropy(
			entropy = (_) { panic("must not request negative entropy") },
		),
	)
})

test("random_bytes_service_error", () {
	{
		assert(
			fail.catch(() { crypto.random_bytes(1) }) ==
				Error(
					process.ProcessError(
						operation = "entropy",
						reason = process.Other(),
						code = "entropy failed",
					),
				),
		)
		()
	} <- (process.Entropy(entropy = (_) { Error("entropy failed") }))
})
