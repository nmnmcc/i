//// Composable two-way JSON codecs.
(
	std/json
	ls = std/list
	ds = std/dict
	(Option, Some, None) = std/option
)

(
	Schema(..)
	SchemaError(..)
	Path(..)
	ParseError(..)
	decode
	encode
	parse
	print
	int
	float
	string
	bool
	null
	literal_string
	literal_int
	optional
	nullable
	list
	dict
	tuple
	object
	field
	optional_field
	union
	tagged_union
	map
	transform
	refine
	lazy
)

Schema(A, E) {
	Schema(read: (json.Json) -> Result(A, E), write: (A) -> json.Json)
}

Path {
	Field(String)
	Index(Int)
}

SchemaError {
	SchemaError(path: List(Path), expected: String)
}

ParseError {
	Syntax(json.JsonError)
	Decode(SchemaError)
}

decode(A, E) = (schema: Schema(A, E), value: json.Json) -> Result(A, E) {
	schema.read(value)
}

encode(A, E) = (schema: Schema(A, E), value: A) -> json.Json {
	schema.write(value)
}

parse(A) = (
	schema: Schema(A, SchemaError)
	text: String
) -> Result(A, ParseError) {
	json.parse(text) |> {
		Error(e) -> Error(Syntax(e))
		Ok(value) -> decode(schema, value) |> {
			Ok(x)    -> Ok(x)
			Error(e) -> Error(Decode(e))
		}
	}
}

print(A, E) = (schema: Schema(A, E), value: A) -> String {
	json.print(encode(schema, value))
}

error = (expected: String) -> SchemaError { SchemaError([], expected) }

at = (part: Path, e: SchemaError) -> SchemaError {
	SchemaError([part, ..e.path], e.expected)
}

int = Schema((x) {
	x |> {
		json.Integer(n) -> Ok(n)
		_               -> Error(error("int"))
	}
}, (x) { json.Integer(x) })

float = Schema((x) {
	x |> {
		json.Number(n) -> Ok(n)
		_              -> Error(error("float"))
	}
}, (x) { json.Number(x) })

string = Schema((x) {
	x |> {
		json.Text(n) -> Ok(n)
		_            -> Error(error("string"))
	}
}, (x) { json.Text(x) })

bool = Schema((x) {
	x |> {
		json.Boolean(n) -> Ok(n)
		_               -> Error(error("bool"))
	}
}, (x) { json.Boolean(x) })

null = Schema((x) {
	x |> {
		json.Null -> Ok(())
		_         -> Error(error("null"))
	}
}, (_) { json.Null })

literal_string = (value: String) { refine(string, (x) { x == value }, value) }

literal_int = (value: Int) { refine(int, (x) { x == value }, "literal int") }

nullable(A) = (s: Schema(A, SchemaError)) -> Schema(Option(A), SchemaError) {
	Schema((x) {
		x |> {
			json.Null -> Ok(None)
			_ -> decode(s, x) |> {
				Ok(v)    -> Ok(Some(v))
				Error(e) -> Error(e)
			}
		}
	}, (x) {
		x |> {
			None    -> json.Null
			Some(v) -> encode(s, v)
		}
	})
}

optional = nullable

list(A) = (s: Schema(A, SchemaError)) -> Schema(List(A), SchemaError) {
	Schema((x) {
		x |> {
			json.Array(xs) -> read_list(s, xs, 0)
			_              -> Error(error("array"))
		}
	}, (xs) { json.Array(ls.map(xs, (v) { encode(s, v) })) })
}

read_list(A) = (
	s: Schema(A, SchemaError)
	xs: List(json.Json)
	index: Int
) -> Result(List(A), SchemaError) {
	xs |> {
		[] -> Ok([])
		[x, ..rest] -> decode(s, x) |> {
			Error(e) -> Error(at(Index(index), e))
			Ok(v) -> read_list(s, rest, index + 1) |> {
				Error(e) -> Error(e)
				Ok(tail) -> Ok([v, ..tail])
			}
		}
	}
}

dict(A) = (
	s: Schema(A, SchemaError)
) -> Schema(ds.Dict(String, A), SchemaError) {
	Schema((x) {
		x |> {
			json.Object(xs) -> read_dict(s, xs)
			_               -> Error(error("object"))
		}
	}, (xs) {
		json.Object(ls.map(ds.to_list(xs), (entry) {
			(k, v) = entry
			(k, encode(s, v))
		}))
	})
}

read_dict(A) = (
	s: Schema(A, SchemaError)
	xs: List((String, json.Json))
) -> Result(ds.Dict(String, A), SchemaError) {
	xs |> {
		[] -> Ok(ds.new())
		[(k, v), ..rest] -> decode(s, v) |> {
			Error(e) -> Error(at(Field(k), e))
			Ok(value) -> read_dict(s, rest) |> {
				Error(e) -> Error(e)
				Ok(tail) -> Ok(ds.insert(tail, k, value))
			}
		}
	}
}

tuple(A, B) = (
	a: Schema(A, SchemaError)
	b: Schema(B, SchemaError)
) -> Schema((A, B), SchemaError) {
	Schema((x) {
		x |> {
			json.Array([one, two]) -> decode(a, one) |> {
				Error(e) -> Error(at(Index(0), e))
				Ok(v) -> decode(b, two) |> {
					Error(e) -> Error(at(Index(1), e))
					Ok(w)    -> Ok((v, w))
				}
			}
			_ -> Error(error("pair"))
		}
	}, (x) {
		(v, w) = x
		json.Array([encode(a, v), encode(b, w)])
	})
}

object(A) = (empty: A) -> Schema(A, SchemaError) {
	Schema((x) {
		x |> {
			json.Object(_) -> Ok(empty)
			_              -> Error(error("object"))
		}
	}, (_) { json.Object([]) })
}

field(A, V) = (
	base: Schema(A, SchemaError)
	key: String
	value: Schema(V, SchemaError)
	get: (A) -> V
	put: (A, V) -> A
) -> Schema(A, SchemaError) {
	Schema((x) {
		decode(base, x) |> {
			Error(e) -> Error(e)
			Ok(r) -> json.field(x, key) |> {
				None -> Error(at(Field(key), error("field")))
				Some(v) -> decode(value, v) |> {
					Error(e) -> Error(at(Field(key), e))
					Ok(y)    -> Ok(put(r, y))
				}
			}
		}
	}, (r) {
		encode(base, r) |> {
			json.Object(entries) -> json.Object(
				ls.append(entries, [(key, encode(value, get(r)))])
			)
			_ -> panic("schema: object builder invariant")
		}
	})
}

optional_field(A, V) = (
	base: Schema(A, SchemaError)
	key: String
	value: Schema(V, SchemaError)
	get: (A) -> Option(V)
	put: (A, Option(V)) -> A
) -> Schema(A, SchemaError) {
	Schema((x) {
		decode(base, x) |> {
			Error(e) -> Error(e)
			Ok(r) -> json.field(x, key) |> {
				None -> Ok(put(r, None))
				Some(v) -> decode(value, v) |> {
					Error(e) -> Error(at(Field(key), e))
					Ok(y)    -> Ok(put(r, Some(y)))
				}
			}
		}
	}, (r) {
		encode(base, r) |> {
			json.Object(entries) -> get(r) |> {
				None -> json.Object(entries)
				Some(v) -> json.Object(
					ls.append(entries, [(key, encode(value, v))])
				)
			}
			_ -> panic("schema: object builder invariant")
		}
	})
}

map(A, B) = (
	s: Schema(A, SchemaError)
	forward: (A) -> B
	backward: (B) -> A
) -> Schema(B, SchemaError) {
	Schema((x) {
		decode(s, x) |> {
			Ok(v)    -> Ok(forward(v))
			Error(e) -> Error(e)
		}
	}, (x) { encode(s, backward(x)) })
}

transform = map

refine(A) = (
	s: Schema(A, SchemaError)
	predicate: (A) -> Bool
	message: String
) -> Schema(A, SchemaError) {
	Schema((x) {
		decode(s, x) |> {
			Error(e) -> Error(e)
			Ok(v) -> predicate(v) |> {
				True  -> Ok(v)
				False -> Error(error(message))
			}
		}
	}, (x) { encode(s, x) })
}

lazy(A) = (get: () -> Schema(A, SchemaError)) -> Schema(A, SchemaError) {
	Schema((x) { decode(get(), x) }, (x) { encode(get(), x) })
}

union(A) = (
	schemas: List(Schema(A, SchemaError))
	choose: (A) -> Int
) -> Schema(A, SchemaError) {
	Schema((x) { read_union(schemas, x) }, (x) {
		ls.at(schemas, choose(x)) |> {
			Some(s) -> encode(s, x)
			None    -> panic("schema: union selector")
		}
	})
}

read_union(A) = (
	schemas: List(Schema(A, SchemaError))
	x: json.Json
) -> Result(A, SchemaError) {
	schemas |> {
		[] -> Error(error("union"))
		[s, ..rest] -> decode(s, x) |> {
			Ok(v)    -> Ok(v)
			Error(_) -> read_union(rest, x)
		}
	}
}

tagged_union(A) = (
	key: String
	cases: List((String, Schema(A, SchemaError)))
	tag: (A) -> String
) -> Schema(A, SchemaError) {
	Schema((x) {
		json.field(x, key) |> {
			Some(json.Text(name)) -> ls.find_map(cases, (entry) {
				(k, s) = entry
				{ k == name } |> {
					True  -> Some(s)
					False -> None
				}
			}) |> {
				Some(s) -> decode(s, x)
				None    -> Error(at(Field(key), error("tag")))
			}
			_ -> Error(at(Field(key), error("tag")))
		}
	}, (x) {
		ls.find_map(cases, (entry) {
			(k, s) = entry
			{ k == tag(x) } |> {
				True  -> Some(s)
				False -> None
			}
		}) |> {
			Some(s) -> encode(s, x) |> {
				json.Object(entries) -> json.Object([
					(key, json.Text(tag(x)))
					..ls.filter(entries, (entry) {
						(k, _) = entry
						k != key
					})
				])
				_ -> panic("schema: tagged union object invariant")
			}
			None -> panic("schema: unknown tag")
		}
	})
}
