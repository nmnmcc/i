(
	i/hostcall
)

(
	main
)

main = () {
	r = hostcall.open_root()
	hostcall.close_root(r)
	hostcall.group_at(r)
}
