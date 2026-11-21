// spec: 9.11
// expect: stop "panic: request: root is gone"
// runtime.md 5.1: a closed Root refuses new groups.
(
	i/request
)

main = () {
	root = request.open_root()
	request.close_root(root)
	request.open_owner() <- (root)
}
