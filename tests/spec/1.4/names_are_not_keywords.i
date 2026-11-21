// spec: 1.4
// expect: pass
// `import`, `export`, `case` and `if` are names, not keywords.
(
	main
)
import = (x) { x }
export = import
case = export
if = case
main = () {
	assert(if(1) == 1)
	()
}
