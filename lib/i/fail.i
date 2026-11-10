//// Errors (§9.3).

(
	raise
	catch
	from_result
	map_error
)

/// Ends the computation with `error`.
@external("ilang:fail", "raise")
raise(E): (error: E) -> Never ? E

/// Changes every error into `Error(…)`.
@external("ilang:fail", "catch")
catch(A, E, N): (body: () -> A ? E ! N) -> Result(A, E) ! N

/// Gives the error `x` for `Error(x)`.
from_result(A, E) = (result: Result(A, E)) -> A ? E {
	result |> {
		Ok(value)    -> value
		Error(error) -> raise(error)
	}
}

/// Changes the errors.
map_error(A, E, N, E2) = (
	body: () -> A ? E ! N
	with: (E) -> E2 ! N
) -> A ? E2 ! N {
	catch(body) |> {
		Ok(value)    -> value
		Error(error) -> raise(with(error))
	}
}
