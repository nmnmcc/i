(
	testlib = (test) = std/test
	schema = (Schema) = std/schema
	std/json
	std/dict
	(Some, None) = std/option
)

round(A) = (s: schema.Schema(A, schema.SchemaError), value: A) -> Bool {
	schema.decode(s, schema.encode(s, value)) == Ok(value)
}

User {
	User(name: String, addresses: List(Address), role: Role)
}

Address {
	Address(city: String)
}

Role {
	Admin
	Member
}

user_schema = () {
	role = schema.tagged_union(
		"kind"
		[
			("admin", schema.map(schema.object(()), (_) { Admin }, (_) { () }))
			("member", schema.map(schema.object(()), (_) { Member }, (_) {
				()
			}))
		]
		(r) {
			r |> {
				Admin  -> "admin"
				Member -> "member"
			}
		}
	)
	address = schema.field(
		schema.object(Address(""))
		"city"
		schema.string
		(a) { a.city }
		(_, city) { Address(city) }
	)
	start = schema.object(User("", [], Member))
	names = schema.field(
		start
		"name"
		schema.string
		(u) { u.name }
		(u, name) { User(name, u.addresses, u.role) }
	)
	addresses = schema.field(
		names
		"addresses"
		schema.list(address)
		(u) { u.addresses }
		(u, a) { User(u.name, a, u.role) }
	)
	schema.field(addresses, "role", role, (u) { u.role }, (u, r) {
		User(u.name, u.addresses, r)
	})
}

test("int_round", () { assert(round(schema.int, 42)) })

test("int_negative", () { assert(round(schema.int, -7)) })

test("int_wrong", () {
	assert(
		schema.decode(schema.int, json.Text("x")) ==
			Error(schema.SchemaError([], "int"))
	)
})

test("float_round", () { assert(round(schema.float, 1.5)) })

test("float_wrong", () {
	assert(
		schema.decode(schema.float, json.Integer(1)) ==
			Error(schema.SchemaError([], "float"))
	)
})

test("string_round", () { assert(round(schema.string, "hello")) })

test("string_empty", () { assert(round(schema.string, "")) })

test("string_wrong", () {
	assert(
		schema.decode(schema.string, json.Null) ==
			Error(schema.SchemaError([], "string"))
	)
})

test("bool_true", () { assert(round(schema.bool, True)) })

test("bool_false", () { assert(round(schema.bool, False)) })

test("bool_wrong", () {
	assert(
		schema.decode(schema.bool, json.Integer(1)) ==
			Error(schema.SchemaError([], "bool"))
	)
})

test("null_round", () { assert(round(schema.null, ())) })

test("null_wrong", () {
	assert(
		schema.decode(schema.null, json.Boolean(False)) ==
			Error(schema.SchemaError([], "null"))
	)
})

test("literal_string_round", () {
	assert(round(schema.literal_string("yes"), "yes"))
})

test("literal_string_wrong", () {
	assert(
		schema.decode(schema.literal_string("yes"), json.Text("no")) ==
			Error(schema.SchemaError([], "yes"))
	)
})

test("literal_int_round", () { assert(round(schema.literal_int(3), 3)) })

test("literal_int_wrong", () {
	assert(
		schema.decode(schema.literal_int(3), json.Integer(4)) ==
			Error(schema.SchemaError([], "literal int"))
	)
})

test("nullable_some", () {
	assert(round(schema.nullable(schema.int), Some(3)))
})

test("nullable_none", () { assert(round(schema.nullable(schema.int), None)) })

test("optional_some", () {
	assert(round(schema.optional(schema.string), Some("a")))
})

test("optional_none", () {
	assert(round(schema.optional(schema.string), None))
})

test("list_round", () { assert(round(schema.list(schema.int), [1, 2, 3])) })

test("list_empty", () { assert(round(schema.list(schema.int), [])) })

test("list_path", () {
	assert(
		schema.decode(
			schema.list(schema.int)
			json.Array([json.Integer(1), json.Text("x")])
		) ==
			Error(schema.SchemaError([schema.Index(1)], "int"))
	)
})

test("dict_round", () {
	assert(round(schema.dict(schema.int), dict.from_list([("x", 1), ("y", 2)])))
})

test("dict_empty", () { assert(round(schema.dict(schema.string), dict.new())) })

test("dict_path", () {
	assert(
		schema.decode(
			schema.dict(schema.bool)
			json.Object([("x", json.Null)])
		) ==
			Error(schema.SchemaError([schema.Field("x")], "bool"))
	)
})

test("tuple_round", () {
	assert(round(schema.tuple(schema.int, schema.string), (1, "a")))
})

test("tuple_path", () {
	assert(
		schema.decode(
			schema.tuple(schema.int, schema.string)
			json.Array([json.Integer(1), json.Null])
		) ==
			Error(schema.SchemaError([schema.Index(1)], "string"))
	)
})

test("map_round", () {
	assert(round(schema.map(schema.int, (x) { x + 1 }, (x) { x - 1 }), 4))
})

test("transform_round", () {
	assert(round(schema.transform(schema.string, (x) { x }, (x) { x }), "x"))
})

test("refine_round", () {
	assert(round(schema.refine(schema.int, (x) { x > 0 }, "positive"), 3))
})

test("refine_error", () {
	assert(
		schema.decode(
			schema.refine(schema.int, (x) { x > 0 }, "positive")
			json.Integer(-1)
		) ==
			Error(schema.SchemaError([], "positive"))
	)
})

test("lazy_round", () { assert(round(schema.lazy(() { schema.int }), 8)) })

test("union_round", () {
	assert(
		round(
			schema.union(
				[
					schema.int
					schema.refine(schema.int, (x) { x > 0 }, "positive")
				]
				(_) { 0 }
			)
			8
		)
	)
})

test("union_order", () {
	assert(
		schema.decode(
			schema.union(
				[schema.refine(schema.int, (x) { x > 5 }, "large"), schema.int]
				(_) { 0 }
			)
			json.Integer(2)
		) ==
			Ok(2)
	)
})

test("union_failure", () {
	assert(
		schema.decode(schema.union([schema.int], (_) { 0 }), json.Null) ==
			Error(schema.SchemaError([], "union"))
	)
})

test("parse_ok", () { assert(schema.parse(schema.int, "123") == Ok(123)) })

test("parse_bad_syntax", () {
	assert(schema.parse(schema.int, "[") |> {
		Error(schema.Syntax(_)) -> True
		_                       -> False
	})
})

test("parse_bad_value", () {
	assert(
		schema.parse(schema.int, "true") ==
			Error(schema.Decode(schema.SchemaError([], "int")))
	)
})

test("print_value", () {
	assert(schema.print(schema.list(schema.int), [1, 2]) == "[1,2]")
})

test("object_empty", () { assert(round(schema.object(()), ())) })

test("nested_round", () {
	assert(
		round(
			user_schema()
			User("Ada", [Address("HK"), Address("Paris")], Admin)
		)
	)
})

test("nested_path", () {
	assert(
		schema.decode(
			user_schema()
			json.Object([
				("name", json.Text("Ada"))
				("addresses", json.Array([json.Object([("city", json.Null)])]))
				("role", json.Object([("kind", json.Text("admin"))]))
			])
		) ==
			Error(
				schema.SchemaError(
					[
						schema.Field("addresses")
						schema.Index(0)
						schema.Field("city")
					]
					"string"
				)
			)
	)
})

test("tag_error", () {
	assert(
		schema.decode(
			user_schema()
			json.Object([
				("name", json.Text("Ada"))
				("addresses", json.Array([]))
				("role", json.Object([("kind", json.Text("other"))]))
			])
		) ==
			Error(
				schema.SchemaError(
					[schema.Field("role"), schema.Field("kind")]
					"tag"
				)
			)
	)
})

user = user_schema()

Schema(DecodedUser, _) = user

test("alias_type", () {
	x: DecodedUser = User("A", [], Admin)
	assert(round(user, x))
})

Tree {
	Tree(value: Int, children: List(Tree))
}

tree_schema = () {
	base = schema.object(Tree(0, []))
	with_value = schema.field(
		base
		"value"
		schema.int
		(t) { t.value }
		(t, v) { Tree(v, t.children) }
	)
	schema.field(
		with_value
		"children"
		schema.list(schema.lazy(() { tree_schema() }))
		(t) { t.children }
		(t, c) { Tree(t.value, c) }
	)
}

test("recursive_round", () {
	assert(round(tree_schema(), Tree(1, [Tree(2, [])])))
})

optional_schema = () {
	schema.optional_field(
		schema.object(None)
		"value"
		schema.int
		(x) { x }
		(_, x) { x }
	)
}

test("optional_field_some", () { assert(round(optional_schema(), Some(5))) })

test("optional_field_none", () { assert(round(optional_schema(), None)) })

test("optional_field_path", () {
	assert(
		schema.decode(optional_schema(), json.Object([("value", json.Null)])) ==
			Error(schema.SchemaError([schema.Field("value")], "int"))
	)
})

test("literal_types", () {
	s: Schema("yes", schema.SchemaError) = schema.literal_string("yes")
	n: Schema(7, schema.SchemaError) = schema.literal_int(7)
	assert(schema.decode(s, json.Text("yes")) == Ok("yes"))
	assert(schema.decode(n, json.Integer(7)) == Ok(7))
})

test("structural_record_round", () {
	base = schema.object((name: "", age: 0))
	names = schema.field(
		base
		"name"
		schema.string
		(r) { r.name }
		(r: (name: String, age: Int), name) { (..r, name: name) }
	)
	person = schema.field(
		names
		"age"
		schema.int
		(r) { r.age }
		(r: (name: String, age: Int), age) { (..r, age: age) }
	)
	assert(round(person, (name: "Ada", age: 36)))
})
