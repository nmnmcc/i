(
	testlib = (test) = std/test
	runtime = review/main
)

test("alias_tests", () { runtime.alias_tests() })

test("fresh_instances", () { runtime.fresh_instances() })

test("inferred_not_known", () { runtime.inferred_not_known() })

test("schema_library", () { runtime.schema_library() })
