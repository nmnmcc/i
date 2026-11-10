// spec: 7.3
// expect: compile-error E0501

// §6.4: parentheses make a tuple; braces group the immediate function call.
s = "b"

x = {
	() {
		"a" = s
		1
	}
}()
