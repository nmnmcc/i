// §4.5 rule 6: A has no input or output occurrence after following fields.
// It is used only in the recursive type argument.
Phantom(A) = (next = () -> Phantom(A))

new = <A>() -> Phantom(A) { Phantom(next = new) }

// §4.7: removing this argument variable does not change accepted programs.
int_phantom = () -> Phantom(Int) { new() }

string_phantom = () -> Phantom(String) { new() }

change = (p: Phantom(Int)) -> Phantom(String) { p }

change_back = (p: Phantom(String)) -> Phantom(Int) { p }
