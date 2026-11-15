(
	i/fs
	i/core
	i/resource
	i/fail
)

(
	counter
	read_lengths
	fs_reads
	add
	compare
	empty
	echo
)

@external("abi:bench", "tick")
tick : (Int) -> Int

@external("abi:bench", "read")
read : (Int) -> BitArray

counter = (n : Int, value : Int) -> Int {
	{ n == 0 } |> (True) { value } | (False) { counter(n - 1, tick(value)) }
}

read_lengths = (n : Int, length : Int) -> Int {
	{ n == 0 } |>
		(True) { length }
		| (False) {
			read_lengths(
				n - 1,
				core.bit_size(read(length)) / 8,
			)
		}
}

add = (n : Int, a : Int, b : Int, last : Int) -> Int {
	{ n == 0 } |> (True) { last } | (False) { add(n - 1, a, b, last + b) }
}

compare = (n : Int, a : Int, b : Int, last : Bool) -> Bool {
	{ n == 0 } |> (True) { last } | (False) { compare(n - 1, a, b, n < b) }
}

empty = (n : Int, a : Int, b : Int, last : Int) -> Int {
	{ n == 0 } |> (True) { last } | (False) { empty(n - 1, a, b, a) }
}

echo = (value : Int) -> Int { tick(value) }

// Real filesystem adapter; /dev/zero keeps every read at eight bytes.
fs_reads = (n : Int, path : String) {
	fail.catch(() {
		resource.acquire(
			() { fs.open(path, fs.ReadOnly) },
			fs.close,
			(file) { fs_loop(n, file) },
		)
	}) |>
		(Ok(answer)) { answer }
		| (Error(_)) { panic("fs read benchmark") }
}

fs_loop = (n : Int, file : fs.File) {
	{ n == 0 } |>
		(True) { 0 }
		| (False) {
			_ = fs.read(file, 8)
			fs_loop(n - 1, file)
		}
}
