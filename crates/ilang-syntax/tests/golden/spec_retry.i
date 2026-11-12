retry = <A, N>(times: Int, body: () <- N -> A) -> A {
	fail.catch(body) |>
		(Ok(value)) { value }
		| (Error(error)) {
			{ times > 1 } |>
				(True) { retry(times - 1, body) }
				| (False) { fail.raise(error) }
		}
}

user = retry(3, () { load_user("1") })
