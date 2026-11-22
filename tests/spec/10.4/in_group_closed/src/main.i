(
	i/hostcall
)

(
	main
)

main = () {
	g = hostcall.group()
	hostcall.close(g)
	hostcall.in_group(g, () { () })
}
