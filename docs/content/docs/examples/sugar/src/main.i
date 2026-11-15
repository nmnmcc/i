(
	i/stdio
	sugar = (if) = hello/sugar
)

unit = (n) { if(n == 1, "minute", "minutes") }
main = () { stdio.print(unit(2)) }
