//// Pure, immutable SplitMix generators (§9.10).
//// Pass the returned generator to the next draw. This is not a cryptographic
//// generator. Split and path streams can have equal outputs or states.

(
	i/core
	i/fail
)

(
	Generator
	RandomError
	InvalidBound(..)
	InvalidPath(..)
	from_seed
	word
	int
	float
	split
	at_path
)

// History is stored newest first. Each step keeps the preceding draw index.
Step = Split | Path

Split {
	Split(Int, Int)
}

Path {
	Path(Int, Int)
}

State {
	State(Int, Int)
}

Generator {
	Generator(Int, State, List(Step), Int)
}

RandomError = InvalidBound | InvalidPath

InvalidBound {
	InvalidBound(Int)
}

InvalidPath {
	InvalidPath(List(Int))
}

word_size = () -> Int { 0x1_0000_0000_0000_0000 }

// SplitMix 0.1.3.1: mix64 is the MurmurHash3 finalizer; mixGamma uses
// mix64variant13. The mixer roles differ from Java's SplittableRandom.
// https://hackage-content.haskell.org/package/splitmix-0.1.3.1/docs/src/System.Random.SplitMix.html
mask = (x: Int) -> Int { core.bit_and(x, 0xffff_ffff_ffff_ffff) }

xor_shift = (x: Int, n: Int) -> Int { core.bit_xor(x, core.shift_right(x, n)) }

mix = (x: Int) -> Int {
	a = mask(xor_shift(mask(x), 33) * 0xff51afd7ed558ccd)
	b = mask(xor_shift(a, 33) * 0xc4ceb9fe1a85ec53)
	xor_shift(b, 33)
}

variant = (x: Int) -> Int {
	a = mask(xor_shift(mask(x), 30) * 0xbf58476d1ce4e5b9)
	b = mask(xor_shift(a, 27) * 0x94d049bb133111eb)
	xor_shift(b, 31)
}

popcount = (x: Int, count: Int) -> Int {
	{ x == 0 } |> {
		True  -> count
		False -> popcount(core.bit_and(x, x - 1), count + 1)
	}
}

mix_gamma = (x: Int) -> Int {
	z = core.bit_or(variant(x), 1)
	{ popcount(xor_shift(z, 1), 0) < 24 } |> {
		True  -> core.bit_xor(z, 0xaaaaaaaaaaaaaaaa)
		False -> z
	}
}

/// Normalizes any seed modulo 2^64; seeds differing by 2^64 are equal.
from_seed = (seed: Int) -> Generator {
	n = mask(seed)
	Generator(n, State(mix(n), mix_gamma(n + 0x9e3779b97f4a7c15)), [], 0)
}

/// One unsigned 64-bit word and the next generator.
word = (g: Generator) -> (Int, Generator) {
	Generator(seed, State(s, gamma), history, draws) = g
	next = mask(s + gamma)
	(mix(next), Generator(seed, State(next, gamma), history, draws + 1))
}

// The least positive power of 2^64 that covers the bound.
space_for = (below: Int, space: Int) -> Int {
	{ space >= below } |> {
		True  -> space
		False -> space_for(below, space * word_size())
	}
}

// Words are assembled least significant first. Thread every rejected draw.
word_group = (
	g: Generator
	space: Int
	scale: Int
	value: Int
) -> (Int, Generator) {
	{ scale == space } |> {
		True -> (value, g)
		False -> {
			(u, next) = word(g)
			word_group(next, space, scale * word_size(), value + u * scale)
		}
	}
}

bounded = (
	g: Generator
	below: Int
	space: Int
	limit: Int
) -> (Int, Generator) {
	(u, next) = word_group(g, space, 1, 0)
	{ u < limit } |> {
		True  -> (u % below, next)
		False -> bounded(next, below, space, limit)
	}
}

/// A value in [0, below), with rejection to avoid modulo bias.
/// InvalidBound carries a nonpositive bound, without drawing a word.
int = (g: Generator, below: Int) -> (Int, Generator) ^ RandomError {
	{ below > 0 } |> {
		True -> {
			space = space_for(below, word_size())
			bounded(g, below, space, space - space % below)
		}
		False -> fail.raise(InvalidBound(below))
	}
}

/// A Float in [0, 1), using the high 53 bits of one word.
float = (g: Generator) -> (Float, Generator) {
	(u, next) = word(g)
	(core.to_float(core.shift_right(u, 11)) /. 9007199254740992.0, next)
}

native_split = (h: State) -> (State, State) {
	State(s, gamma) = h
	a = mask(s + gamma)
	b = mask(a + gamma)
	(State(b, gamma), State(mix(a), mix_gamma(b)))
}

// An edge selects a new child, never the parent's continuation stream.
edge = (h: State, bit: Int) -> State {
	(rest, a) = native_split(h)
	{ bit == 0 } |> {
		True -> a
		False -> {
			(_, b) = native_split(rest)
			b
		}
	}
}

/// Two tagged streams, at split-tree codes 00 and 01.
split = (g: Generator) -> (Generator, Generator) {
	Generator(seed, state, history, draws) = g
	h = edge(state, 0)
	(
		Generator(seed, edge(h, 0), [Split(1, draws), ..history], 0)
		Generator(seed, edge(h, 1), [Split(2, draws), ..history], 0)
	)
}

valid_path = (path: List(Int)) -> Bool {
	path |> {
		[]          -> True
		[i, ..rest] -> i > 0 && valid_path(rest)
	}
}

// Produce the positive index's binary digits in most-significant-first order.
binary = (n: Int, bits: List(Int), length: Int) -> (List(Int), Int) {
	{ n == 0 } |> {
		True  -> (bits, length)
		False -> binary(n / 2, [n % 2, ..bits], length + 1)
	}
}

zeros = (h: State, count: Int) -> State {
	{ count == 0 } |> {
		True  -> h
		False -> zeros(edge(h, 0), count - 1)
	}
}

follow = (h: State, bits: List(Int)) -> State {
	bits |> {
		[]            -> h
		[bit, ..rest] -> follow(edge(h, bit), rest)
	}
}

derive_path = (g: Generator, path: List(Int)) -> Generator {
	path |> {
		[] -> g
		[i, ..rest] -> {
			Generator(seed, state, history, draws) = g
			(bits, length) = binary(i, [], 0)
			h = follow(zeros(edge(state, 1), length - 1), bits)
			derive_path(
				Generator(seed, h, [Path(i, draws), ..history], 0)
				rest
			)
		}
	}
}

/// Derives positive path indices. Empty paths preserve the complete generator.
/// InvalidPath carries the whole supplied path, without deriving or drawing.
at_path = (g: Generator, path: List(Int)) -> Generator ^ RandomError {
	valid_path(path) |> {
		True  -> derive_path(g, path)
		False -> fail.raise(InvalidPath(path))
	}
}
