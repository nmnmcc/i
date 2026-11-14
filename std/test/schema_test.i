(
	testlib = (test) = std/test
	schema = (Schema) = std/schema
	std/json
	std/dict
	(Some, None) = std/option
)

round = <A>(s : schema.Schema(A, schema.SchemaError), value : A) -> Bool {
	schema.decode(s, schema.encode(s, value)) == Ok(value)
}

User = (name = String, addresses = List(Address), role = Role)

Address = (city = String)

Admin = ()
Member = ()
Role = Admin | Member

user_schema = () {
	role = schema.tagged_union(
		"kind",
		[
			("admin", schema.map(schema.object(()), (_) { Admin }, (_) { () })),
			(
				"member",
				schema.map(schema.object(()), (_) { Member }, (_) { () }),
			),
		],
		(r) { r |> (Admin) { "admin" } | (Member) { "member" } },
	)
	address = schema.field(
		schema.object(Address(city = "")),
		"city",
		schema.string,
		(a) { a.city },
		(_, city) { Address(city = city) },
	)
	start = schema.object(User(name = "", addresses = [], role = Member))
	names = schema.field(
		start,
		"name",
		schema.string,
		(u) { u.name },
		(u, name) { User(name = name, addresses = u.addresses, role = u.role) },
	)
	addresses = schema.field(
		names,
		"addresses",
		schema.list(address),
		(u) { u.addresses },
		(u, a) { User(name = u.name, addresses = a, role = u.role) },
	)
	schema.field(
		addresses,
		"role",
		role,
		(u) { u.role },
		(u, r) { User(name = u.name, addresses = u.addresses, role = r) },
	)
}

test("int_round", () { assert(round(schema.int, 42)) })

test("int_negative", () { assert(round(schema.int, -7)) })

test("int_wrong", () {
	assert(
		schema.decode(schema.int, json.Text("x")) ==
			Error(schema.SchemaError(path = [], expected = "int")),
	)
})

test("float_round", () { assert(round(schema.float, 1.5)) })

test("float_wrong", () {
	assert(
		schema.decode(schema.float, json.Integer(1)) ==
			Error(schema.SchemaError(path = [], expected = "float")),
	)
})

test("string_round", () { assert(round(schema.string, "hello")) })

test("string_empty", () { assert(round(schema.string, "")) })

test("string_wrong", () {
	assert(
		schema.decode(schema.string, json.Null) ==
			Error(schema.SchemaError(path = [], expected = "string")),
	)
})

test("bool_true", () { assert(round(schema.bool, True)) })

test("bool_false", () { assert(round(schema.bool, False)) })

test("bool_wrong", () {
	assert(
		schema.decode(schema.bool, json.Integer(1)) ==
			Error(schema.SchemaError(path = [], expected = "bool")),
	)
})

test("null_round", () { assert(round(schema.null, ())) })

test("null_wrong", () {
	assert(
		schema.decode(schema.null, json.Boolean(False)) ==
			Error(schema.SchemaError(path = [], expected = "null")),
	)
})

test("literal_string_round", () {
	assert(
		round(schema.literal_string("yes"), "yes"),
	)
})

test("literal_string_wrong", () {
	assert(
		schema.decode(schema.literal_string("yes"), json.Text("no")) ==
			Error(schema.SchemaError(path = [], expected = "yes")),
	)
})

test("literal_int_round", () {
	assert(
		round(schema.literal_int(3), 3),
	)
})

test("literal_int_wrong", () {
	assert(
		schema.decode(schema.literal_int(3), json.Integer(4)) ==
			Error(schema.SchemaError(path = [], expected = "literal int")),
	)
})

test("nullable_some", () {
	assert(
		round(schema.nullable(schema.int), Some(3)),
	)
})

test("nullable_none", () {
	assert(
		round(schema.nullable(schema.int), None),
	)
})

test("optional_some", () {
	assert(
		round(schema.optional(schema.string), Some("a")),
	)
})

test("optional_none", () {
	assert(
		round(schema.optional(schema.string), None),
	)
})

test("list_round", () {
	assert(
		round(schema.list(schema.int), [1, 2, 3]),
	)
})

test("list_empty", () {
	assert(
		round(schema.list(schema.int), []),
	)
})

test("list_path", () {
	assert(
		schema.decode(
			schema.list(schema.int),
			json.Array([json.Integer(1), json.Text("x")]),
		) ==
			Error(
				schema.SchemaError(path = [schema.Index(1)], expected = "int"),
			),
	)
})

test("dict_round", () {
	assert(
		round(schema.dict(schema.int), dict.from_list([("x", 1), ("y", 2)])),
	)
})

test("dict_empty", () {
	assert(
		round(schema.dict(schema.string), dict.new()),
	)
})

test("dict_path", () {
	assert(
		schema.decode(
			schema.dict(schema.bool),
			json.Object([("x", json.Null)]),
		) ==
			Error(
				schema.SchemaError(
					path = [schema.Field("x")],
					expected = "bool",
				),
			),
	)
})

test("tuple_round", () {
	assert(
		round(schema.tuple(schema.int, schema.string), (1, "a")),
	)
})

test("tuple_path", () {
	assert(
		schema.decode(
			schema.tuple(schema.int, schema.string),
			json.Array([json.Integer(1), json.Null]),
		) ==
			Error(
				schema.SchemaError(
					path = [schema.Index(1)],
					expected = "string",
				),
			),
	)
})

test("map_round", () {
	assert(
		round(schema.map(schema.int, (x) { x + 1 }, (x) { x - 1 }), 4),
	)
})

test("transform_round", () {
	assert(
		round(schema.transform(schema.string, (x) { x }, (x) { x }), "x"),
	)
})

test("refine_round", () {
	assert(
		round(schema.refine(schema.int, (x) { x > 0 }, "positive"), 3),
	)
})

test("refine_error", () {
	assert(
		schema.decode(
			schema.refine(schema.int, (x) { x > 0 }, "positive"),
			json.Integer(-1),
		) ==
			Error(schema.SchemaError(path = [], expected = "positive")),
	)
})

test("lazy_round", () {
	assert(
		round(schema.lazy(() { schema.int }), 8),
	)
})

test("union_round", () {
	assert(
		round(
			schema.union([
				schema.int,
				schema.refine(schema.int, (x) { x > 0 }, "positive"),
			], (_) { 0 }),
			8,
		),
	)
})

test("union_order", () {
	assert(
		schema.decode(
			schema.union([
				schema.refine(schema.int, (x) { x > 5 }, "large"),
				schema.int,
			], (_) { 0 }),
			json.Integer(2),
		) ==
			Ok(2),
	)
})

test("union_failure", () {
	assert(
		schema.decode(schema.union([schema.int], (_) { 0 }), json.Null) ==
			Error(schema.SchemaError(path = [], expected = "union")),
	)
})

test("parse_ok", () { assert(schema.parse(schema.int, "123") == Ok(123)) })

test("parse_bad_syntax", () {
	assert(
		schema.parse(schema.int, "[") |>
			(Error(schema.Syntax(_))) { True }
			| (_) { False },
	)
})

test("parse_bad_value", () {
	assert(
		schema.parse(schema.int, "true") ==
			Error(
				schema.Decode(schema.SchemaError(path = [], expected = "int")),
			),
	)
})

test("print_value", () {
	assert(
		schema.print(schema.list(schema.int), [1, 2]) == "[1,2]",
	)
})

test("object_empty", () {
	assert(
		round(schema.object(()), ()),
	)
})

test("nested_round", () {
	assert(
		round(
			user_schema(),
			User(
				name = "Ada",
				addresses = [Address(city = "HK"), Address(city = "Paris")],
				role = Admin,
			),
		),
	)
})

test("nested_path", () {
	assert(
		schema.decode(
			user_schema(),
			json.Object(
				[
					("name", json.Text("Ada")),
					(
						"addresses",
						json.Array([json.Object([("city", json.Null)])]),
					),
					("role", json.Object([("kind", json.Text("admin"))])),
				],
			),
		) ==
			Error(
				schema.SchemaError(
					path = [
						schema.Field("addresses"),
						schema.Index(0),
						schema.Field("city"),
					],
					expected = "string",
				),
			),
	)
})

test("tag_error", () {
	assert(
		schema.decode(
			user_schema(),
			json.Object(
				[
					("name", json.Text("Ada")),
					("addresses", json.Array([])),
					("role", json.Object([("kind", json.Text("other"))])),
				],
			),
		) ==
			Error(
				schema.SchemaError(
					path = [schema.Field("role"), schema.Field("kind")],
					expected = "tag",
				),
			),
	)
})

user = user_schema()

Schema(DecodedUser, _) = user

test("alias_type", () {
	x : DecodedUser = User(name = "A", addresses = [], role = Admin)
	assert(round(user, x))
})

Tree = (value = Int, children = List(Tree))

tree_schema = () {
	base = schema.object(Tree(value = 0, children = []))
	with_value = schema.field(
		base,
		"value",
		schema.int,
		(t) { t.value },
		(t, v) { Tree(value = v, children = t.children) },
	)
	schema.field(
		with_value,
		"children",
		schema.list(schema.lazy(() { tree_schema() })),
		(t) { t.children },
		(t, c) { Tree(value = t.value, children = c) },
	)
}

test("recursive_round", () {
	assert(
		round(
			tree_schema(),
			Tree(value = 1, children = [Tree(value = 2, children = [])]),
		),
	)
})

optional_schema = () {
	schema.optional_field(
		schema.object(None),
		"value",
		schema.int,
		(x) { x },
		(_, x) { x },
	)
}

test("optional_field_some", () {
	assert(
		round(optional_schema(), Some(5)),
	)
})

test("optional_field_none", () {
	assert(
		round(optional_schema(), None),
	)
})

test("optional_field_path", () {
	assert(
		schema.decode(optional_schema(), json.Object([("value", json.Null)])) ==
			Error(
				schema.SchemaError(
					path = [schema.Field("value")],
					expected = "int",
				),
			),
	)
})

test("literal_types", () {
	s : Schema("yes", schema.SchemaError) = schema.literal_string("yes")
	n : Schema(7, schema.SchemaError) = schema.literal_int(7)
	assert(schema.decode(s, json.Text("yes")) == Ok("yes"))
	assert(schema.decode(n, json.Integer(7)) == Ok(7))
})

test("structural_record_round", () {
	base = schema.object((name = "", age = 0))
	names = schema.field(
		base,
		"name",
		schema.string,
		(r) { r.name },
		(r : (name = String, age = Int), name) { (..r, name = name) },
	)
	person = schema.field(
		names,
		"age",
		schema.int,
		(r) { r.age },
		(r : (name = String, age = Int), age) { (..r, age = age) },
	)
	assert(round(person, (name = "Ada", age = 36)))
})
