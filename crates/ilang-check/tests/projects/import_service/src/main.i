(
	log = app/log
)

(
	main
	tick
)

@import("app:raw/v1", "tick", "later")
tick : (Int) -> Int

main = () {
	logger = log.make_logger()
	clock = log.make_clock()
	clock.now()
}
