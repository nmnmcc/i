// spec: 10.4
// expect: compile-error E0323
// host-boundary §5.1: `i/hostcall` needs `hostcall = true` in the importing
// package's i.toml; the message names i.toml.
(
	i/hostcall
)

main = () { hostcall.close(hostcall.group()) }
