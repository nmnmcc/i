(
	svc = app/svc
)

(
	main
)

main = () {
	got = { svc.twice() } <- (svc.Svc(ping = () { 21 }))
	assert(got == 42)
}
