//// SHA-2 digests and key derivation in ordinary immutable i.
//// Digest inputs are bit strings (SHA padding uses their exact bit length).
//// HMAC and KDF inputs must be whole bytes; errors are values.
//// Hex codecs already live in std/bit_array.
(
	i/core
	i/fail
	i/process
	i/request
	std/list
	std/bit_array
	(Some, None) = std/option
)

(
	CryptoError(..)
	sha256
	sha512
	hmac_sha256
	hmac_sha512
	hkdf_sha256
	hkdf_sha512
	hkdf_extract_sha256
	hkdf_extract_sha512
	hkdf_expand_sha256
	hkdf_expand_sha512
	pbkdf2_sha256
	constant_time_equal
	random_bytes
)

InvalidLength = ()

InvalidIterations = ()
UnalignedInput = ()
CryptoError = InvalidLength | InvalidIterations | UnalignedInput
xor3 = (a : Int, b : Int, c : Int) -> Int {
	core.bit_xor(core.bit_xor(a, b), c)
}

word = (x : Int, width : Int) -> Int {
	core.bit_and(x, core.shift_left(1, width) - 1)
}

rotate = (x : Int, n : Int, width : Int) -> Int {
	core.bit_or(
		core.shift_right(x, n),
		word(core.shift_left(x, width - n), width),
	)
}

nth = (xs : List(Int), n : Int) -> Int {
	list.at(xs, n) |>
		(Some(x)) { x }
		| (None) { panic("crypto: schedule index") }
}

Words = (Int, Int, Int, Int, Int, Int, Int, Int)

add_words = (x : Words, y : Words, width : Int) -> Words {
	Words(a, b, c, d, e, f, g, h) = x
	Words(i, j, k, l, m, n, o, p) = y
	Words(
		word(a + i, width),
		word(b + j, width),
		word(c + k, width),
		word(d + l, width),
		word(e + m, width),
		word(f + n, width),
		word(g + o, width),
		word(h + p, width),
	)
}

serialize = (x : Words, width : Int) -> BitArray {
	Words(a, b, c, d, e, f, g, h) = x
	<<
		a:u(width),
		b:u(width),
		c:u(width),
		d:u(width),
		e:u(width),
		f:u(width),
		g:u(width),
		h:u(width),
	>>
}

constants32 = [
	0x428a2f98,
	0x71374491,
	0xb5c0fbcf,
	0xe9b5dba5,
	0x3956c25b,
	0x59f111f1,
	0x923f82a4,
	0xab1c5ed5,
	0xd807aa98,
	0x12835b01,
	0x243185be,
	0x550c7dc3,
	0x72be5d74,
	0x80deb1fe,
	0x9bdc06a7,
	0xc19bf174,
	0xe49b69c1,
	0xefbe4786,
	0x0fc19dc6,
	0x240ca1cc,
	0x2de92c6f,
	0x4a7484aa,
	0x5cb0a9dc,
	0x76f988da,
	0x983e5152,
	0xa831c66d,
	0xb00327c8,
	0xbf597fc7,
	0xc6e00bf3,
	0xd5a79147,
	0x06ca6351,
	0x14292967,
	0x27b70a85,
	0x2e1b2138,
	0x4d2c6dfc,
	0x53380d13,
	0x650a7354,
	0x766a0abb,
	0x81c2c92e,
	0x92722c85,
	0xa2bfe8a1,
	0xa81a664b,
	0xc24b8b70,
	0xc76c51a3,
	0xd192e819,
	0xd6990624,
	0xf40e3585,
	0x106aa070,
	0x19a4c116,
	0x1e376c08,
	0x2748774c,
	0x34b0bcb5,
	0x391c0cb3,
	0x4ed8aa4a,
	0x5b9cca4f,
	0x682e6ff3,
	0x748f82ee,
	0x78a5636f,
	0x84c87814,
	0x8cc70208,
	0x90befffa,
	0xa4506ceb,
	0xbef9a3f7,
	0xc67178f2,
]

/// The 256-bit SHA-2 digest. For example `sha256(<<"abc":utf8>>)`.
sha256 = (input : BitArray) -> BitArray {
	n = core.bit_size(input)
	zeros = { 512 - { n + 1 + 64 } % 512 } % 512
	padded = <<input:bits, 1:u1, 0:u(zeros), n:u(64)>>
	serialize(
		blocks32(
			padded,
			Words(
				0x6a09e667,
				0xbb67ae85,
				0x3c6ef372,
				0xa54ff53a,
				0x510e527f,
				0x9b05688c,
				0x1f83d9ab,
				0x5be0cd19,
			),
		),
		32,
	)
}

blocks32 = (input : BitArray, state : Words) -> Words {
	input |>
		(<<>>) { state }
		| (<<block:bits(512), tail:bits>>) {
			blocks32(
				tail,
				add_words(state, rounds32(block, constants32, [], state), 32),
			)
		}
		| (_) { panic("crypto: padded block size") }
}

rounds32 = (
	block : BitArray,
	constants : List(Int),
	schedule : List(Int),
	state : Words
) -> Words {
	constants |>
		([]) { state }
		| ([k, ..ks]) {
			(v, tail) = block |>
				(<<v:u(32), tail:bits>>) { (v, tail) }
				| (<<>>) {
					x = nth(schedule, 14)
					y = nth(schedule, 1)
					s0 = xor3(
						rotate(x, 7, 32),
						rotate(x, 18, 32),
						core.shift_right(x, 3),
					)
					s1 = xor3(
						rotate(y, 17, 32),
						rotate(y, 19, 32),
						core.shift_right(y, 10),
					)
					(
						word(
							nth(schedule, 15) + s0 + nth(schedule, 6) + s1,
							32,
						),
						<<>>,
					)
				}
				| (_) { panic("crypto: word size") }
			Words(a, b, c, d, e, f, g, h) = state
			ch = core.bit_xor(
				core.bit_and(e, f),
				core.bit_and(core.bit_not(e), g),
			)
			maj = xor3(
				core.bit_and(a, b),
				core.bit_and(a, c),
				core.bit_and(b, c),
			)
			t1 = h +
				xor3(rotate(e, 6, 32), rotate(e, 11, 32), rotate(e, 25, 32)) +
				ch +
				k +
				v
			t2 = xor3(rotate(a, 2, 32), rotate(a, 13, 32), rotate(a, 22, 32)) +
				maj
			rounds32(
				tail,
				ks,
				[v, ..schedule],
				Words(word(t1 + t2, 32), a, b, c, word(d + t1, 32), e, f, g),
			)
		}
}

constants64 = [
	0x428a2f98d728ae22,
	0x7137449123ef65cd,
	0xb5c0fbcfec4d3b2f,
	0xe9b5dba58189dbbc,
	0x3956c25bf348b538,
	0x59f111f1b605d019,
	0x923f82a4af194f9b,
	0xab1c5ed5da6d8118,
	0xd807aa98a3030242,
	0x12835b0145706fbe,
	0x243185be4ee4b28c,
	0x550c7dc3d5ffb4e2,
	0x72be5d74f27b896f,
	0x80deb1fe3b1696b1,
	0x9bdc06a725c71235,
	0xc19bf174cf692694,
	0xe49b69c19ef14ad2,
	0xefbe4786384f25e3,
	0x0fc19dc68b8cd5b5,
	0x240ca1cc77ac9c65,
	0x2de92c6f592b0275,
	0x4a7484aa6ea6e483,
	0x5cb0a9dcbd41fbd4,
	0x76f988da831153b5,
	0x983e5152ee66dfab,
	0xa831c66d2db43210,
	0xb00327c898fb213f,
	0xbf597fc7beef0ee4,
	0xc6e00bf33da88fc2,
	0xd5a79147930aa725,
	0x06ca6351e003826f,
	0x142929670a0e6e70,
	0x27b70a8546d22ffc,
	0x2e1b21385c26c926,
	0x4d2c6dfc5ac42aed,
	0x53380d139d95b3df,
	0x650a73548baf63de,
	0x766a0abb3c77b2a8,
	0x81c2c92e47edaee6,
	0x92722c851482353b,
	0xa2bfe8a14cf10364,
	0xa81a664bbc423001,
	0xc24b8b70d0f89791,
	0xc76c51a30654be30,
	0xd192e819d6ef5218,
	0xd69906245565a910,
	0xf40e35855771202a,
	0x106aa07032bbd1b8,
	0x19a4c116b8d2d0c8,
	0x1e376c085141ab53,
	0x2748774cdf8eeb99,
	0x34b0bcb5e19b48a8,
	0x391c0cb3c5c95a63,
	0x4ed8aa4ae3418acb,
	0x5b9cca4f7763e373,
	0x682e6ff3d6b2b8a3,
	0x748f82ee5defb2fc,
	0x78a5636f43172f60,
	0x84c87814a1f0ab72,
	0x8cc702081a6439ec,
	0x90befffa23631e28,
	0xa4506cebde82bde9,
	0xbef9a3f7b2c67915,
	0xc67178f2e372532b,
	0xca273eceea26619c,
	0xd186b8c721c0c207,
	0xeada7dd6cde0eb1e,
	0xf57d4f7fee6ed178,
	0x06f067aa72176fba,
	0x0a637dc5a2c898a6,
	0x113f9804bef90dae,
	0x1b710b35131c471b,
	0x28db77f523047d84,
	0x32caab7b40c72493,
	0x3c9ebe0a15c9bebc,
	0x431d67c49c100d4c,
	0x4cc5d4becb3e42b6,
	0x597f299cfc657e2a,
	0x5fcb6fab3ad6faec,
	0x6c44198c4a475817,
]

/// The 512-bit SHA-2 digest. For example `sha512(<<"abc":utf8>>)`.
sha512 = (input : BitArray) -> BitArray {
	n = core.bit_size(input)
	zeros = { 1024 - { n + 1 + 128 } % 1024 } % 1024
	padded = <<input:bits, 1:u1, 0:u(zeros), n:u(128)>>
	serialize(
		blocks64(
			padded,
			Words(
				0x6a09e667f3bcc908,
				0xbb67ae8584caa73b,
				0x3c6ef372fe94f82b,
				0xa54ff53a5f1d36f1,
				0x510e527fade682d1,
				0x9b05688c2b3e6c1f,
				0x1f83d9abfb41bd6b,
				0x5be0cd19137e2179,
			),
		),
		64,
	)
}

blocks64 = (input : BitArray, state : Words) -> Words {
	input |>
		(<<>>) { state }
		| (<<block:bits(1024), tail:bits>>) {
			blocks64(
				tail,
				add_words(state, rounds64(block, constants64, [], state), 64),
			)
		}
		| (_) { panic("crypto: padded block size") }
}

rounds64 = (
	block : BitArray,
	constants : List(Int),
	schedule : List(Int),
	state : Words
) -> Words {
	constants |>
		([]) { state }
		| ([k, ..ks]) {
			(v, tail) = block |>
				(<<v:u(64), tail:bits>>) { (v, tail) }
				| (<<>>) {
					x = nth(schedule, 14)
					y = nth(schedule, 1)
					s0 = xor3(
						rotate(x, 1, 64),
						rotate(x, 8, 64),
						core.shift_right(x, 7),
					)
					s1 = xor3(
						rotate(y, 19, 64),
						rotate(y, 61, 64),
						core.shift_right(y, 6),
					)
					(
						word(
							nth(schedule, 15) + s0 + nth(schedule, 6) + s1,
							64,
						),
						<<>>,
					)
				}
				| (_) { panic("crypto: word size") }
			Words(a, b, c, d, e, f, g, h) = state
			ch = core.bit_xor(
				core.bit_and(e, f),
				core.bit_and(core.bit_not(e), g),
			)
			maj = xor3(
				core.bit_and(a, b),
				core.bit_and(a, c),
				core.bit_and(b, c),
			)
			t1 = h +
				xor3(rotate(e, 14, 64), rotate(e, 18, 64), rotate(e, 41, 64)) +
				ch +
				k +
				v
			t2 = xor3(rotate(a, 28, 64), rotate(a, 34, 64), rotate(a, 39, 64)) +
				maj
			rounds64(
				tail,
				ks,
				[v, ..schedule],
				Words(word(t1 + t2, 64), a, b, c, word(d + t1, 64), e, f, g),
			)
		}
}

aligned = (xs : List(BitArray)) -> Bool {
	list.all(xs, (x) { core.bit_size(x) % 8 == 0 })
}

key_pad = (
	key : BitArray,
	block : Int,
	hash : (BitArray) -> BitArray
) -> BitArray {
	k = { bit_array.byte_size(key) > block } |>
		(True) { hash(key) }
		| (False) { key }
	<<k:bytes, 0:u({ block - bit_array.byte_size(k) } * 8)>>
}

xor_byte = (bytes : BitArray, mask : Int) -> BitArray {
	go = (xs, acc) {
		xs |>
			(<<>>) { bit_array.concat(list.reverse(acc)) }
			| (<<x, rest:bytes>>) {
				go(rest, [<<{ core.bit_xor(x, mask) }>>, ..acc])
			}
			| (_) { panic("crypto: padded key alignment") }
	}
	go(bytes, [])
}

// A prepared HMAC stores immutable pads, reusable across KDF iterations.
Mac = (BitArray, BitArray, (BitArray) -> BitArray)

prepare = (key : BitArray, block : Int, hash : (BitArray) -> BitArray) -> Mac {
	pad = key_pad(key, block, hash)
	Mac(xor_byte(pad, 54), xor_byte(pad, 92), hash)
}

authenticate = (mac : Mac, input : BitArray) -> BitArray {
	Mac(inner, outer, hash) = mac
	digest = hash(<<inner:bytes, input:bytes>>)
	hash(<<outer:bytes, digest:bytes>>)
}

hmac = (
	key : BitArray,
	input : BitArray,
	block : Int,
	hash : (BitArray) -> BitArray
) -> Result(BitArray, CryptoError) {
	aligned([key, input]) |>
		(False) { Error(UnalignedInput) }
		| (True) { Ok(authenticate(prepare(key, block, hash), input)) }
}

/// RFC 4231 HMAC-SHA-256: `hmac_sha256(<<1>>, <<2>>)`.
hmac_sha256 = (key : BitArray, input : BitArray) -> Result(
	BitArray,
	CryptoError,
) { hmac(key, input, 64, sha256) }

/// RFC 4231 HMAC-SHA-512, with arbitrary-length byte keys.
hmac_sha512 = (key : BitArray, input : BitArray) -> Result(
	BitArray,
	CryptoError,
) { hmac(key, input, 128, sha512) }

/// RFC 5869 extract. Empty salt is equivalent to a hash-length zero salt.
hkdf_extract_sha256 = (salt : BitArray, input : BitArray) -> Result(
	BitArray,
	CryptoError,
) { hmac_sha256(salt, input) }

/// RFC 5869 extract using SHA-512.
hkdf_extract_sha512 = (salt : BitArray, input : BitArray) -> Result(
	BitArray,
	CryptoError,
) { hmac_sha512(salt, input) }

expand = (
	prk : BitArray,
	info : BitArray,
	length : Int,
	size : Int,
	block : Int,
	hash : (BitArray) -> BitArray
) -> Result(BitArray, CryptoError) {
	{ length < 0 || length > 255 * size || bit_array.byte_size(prk) < size } |>
		(True) { Error(InvalidLength) }
		| (False) {
			aligned([prk, info]) |>
				(False) { Error(UnalignedInput) }
				| (True) {
					mac = prepare(prk, block, hash)
					go = (left, counter, previous, acc) {
						{ left <= 0 } |>
							(True) { bit_array.concat(list.reverse(acc)) }
							| (False) {
								t = authenticate(
									mac,
									<<previous:bytes, info:bytes, counter>>,
								)
								n = { left < size } |>
									(True) { left }
									| (False) { size }
								go(
									left - n,
									counter + 1,
									t,
									[<<t:bytes(n)>>, ..acc],
								)
							}
					}
					Ok(go(length, 1, <<>>, []))
				}
		}
}

/// Expands a SHA-256 PRK into 0..8160 bytes. A short PRK is an error.
hkdf_expand_sha256 = (prk : BitArray, info : BitArray, length : Int) -> Result(
	BitArray,
	CryptoError,
) { expand(prk, info, length, 32, 64, sha256) }

/// Expands a SHA-512 PRK into 0..16320 bytes.
hkdf_expand_sha512 = (prk : BitArray, info : BitArray, length : Int) -> Result(
	BitArray,
	CryptoError,
) { expand(prk, info, length, 64, 128, sha512) }

/// Extracts then expands: `hkdf_sha256(key, <<>>, <<>>, 32)`.
hkdf_sha256 = (
	input : BitArray,
	salt : BitArray,
	info : BitArray,
	length : Int
) -> Result(BitArray, CryptoError) {
	hkdf_extract_sha256(salt, input) |>
		(Error(e)) { Error(e) }
		| (Ok(prk)) { hkdf_expand_sha256(prk, info, length) }
}

/// HKDF with SHA-512.
hkdf_sha512 = (
	input : BitArray,
	salt : BitArray,
	info : BitArray,
	length : Int
) -> Result(BitArray, CryptoError) {
	hkdf_extract_sha512(salt, input) |>
		(Error(e)) { Error(e) }
		| (Ok(prk)) { hkdf_expand_sha512(prk, info, length) }
}

xor_bits = (a : BitArray, b : BitArray) -> BitArray {
	n = core.bit_size(a)
	(a, b) |>
		((<<x:u(n)>>, <<y:u(n)>>)) { <<{ core.bit_xor(x, y) }:u(n)>> }
		| (_) { panic("crypto: equal digest lengths") }
}

/// PBKDF2-HMAC-SHA-256, positive iteration count and 0..(2^32-1)*32 bytes.
/// `pbkdf2_sha256(<<"password":utf8>>, <<"salt":utf8>>, 1, 32)`.
pbkdf2_sha256 = (
	password : BitArray,
	salt : BitArray,
	iterations : Int,
	length : Int
) -> Result(BitArray, CryptoError) {
	{ iterations <= 0 } |>
		(True) { Error(InvalidIterations) }
		| (False) {
			{ length < 0 || length > 0xffff_ffff * 32 } |>
				(True) { Error(InvalidLength) }
				| (False) {
					aligned([password, salt]) |>
						(False) { Error(UnalignedInput) }
						| (True) {
							mac = prepare(password, 64, sha256)
							iterate = (n, previous, acc) {
								{ n == 0 } |>
									(True) { acc }
									| (False) {
										next = authenticate(mac, previous)
										iterate(
											n - 1,
											next,
											xor_bits(acc, next),
										)
									}
							}
							go = (left, index, acc) {
								{ left <= 0 } |>
									(True) {
										bit_array.concat(list.reverse(acc))
									}
									| (False) {
										first = authenticate(
											mac,
											<<salt:bytes, index:u(32)>>,
										)
										value = iterate(
											iterations - 1,
											first,
											first,
										)
										n = { left < 32 } |>
											(True) { left }
											| (False) { 32 }
										go(
											left - n,
											index + 1,
											[<<value:bytes(n)>>, ..acc],
										)
									}
							}
							Ok(go(length, 1, []))
						}
				}
		}
}

/// Compares every bit for equal-length inputs, without a content-dependent exit.
/// Length is public. This source-level timing property is not a host timing guarantee.
constant_time_equal = (a : BitArray, b : BitArray) -> Bool {
	{ core.bit_size(a) != core.bit_size(b) } |>
		(True) { False }
		| (False) {
			go = (x, y, diff) {
				(x, y) |>
					((<<>>, <<>>)) { diff == 0 }
					| ((<<u, xs:bits>>, <<v, ys:bits>>)) {
						go(
							xs,
							ys,
							core.bit_or(diff, core.bit_xor(u, v)),
						)
					}
					| (_) {
						n = core.bit_size(x)
						(x, y) |>
							((<<u:u(n)>>, <<v:u(n)>>)) {
								core.bit_or(diff, core.bit_xor(u, v)) == 0
							}
							| (_) { panic("crypto: equality length invariant") }
					}
			}
			go(a, b, 0)
		}
}

/// Secure bytes from the existing Entropy service. No pseudo-random substitute.
random_bytes = (length : Int) -> Result(BitArray, CryptoError) {
	{ length < 0 } |>
		(True) { Error(InvalidLength) }
		| (False) { Ok(process.entropy(length)) }
}
