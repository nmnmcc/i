// spec: 9.10
// expect: pass

(
	i/random
)

// Keep aliases unannotated so --types reports the inferred public schemes.
from_seed = random.from_seed

word = random.word

int = random.int

float = random.float

split = random.split

at_path = random.at_path

// §9.10's parameter labels and the absence of a service are public contracts.
// This top-level call needs no runner or host import.
initial = from_seed(seed: 1337)

first = word(g: initial)

@test
public_api = () {
	assert(first.0 == 0xb5c19e300e8b07b3)
	assert(int(g: initial, below: 1).0 == 0)
	assert(float(g: initial).1 == first.1)
	assert(split(g: initial).0 != split(g: initial).1)
	assert(at_path(g: initial, path: []) == initial)
	Nil
}
