//// Source syntax values for compile-time programs and tests.
(
	i/fail
)

(
	Syntax
	Node(..)
	TypeView(..)
	Name(..)
	Discard(..)
	Up(..)
	Symbol(..)
	Text(..)
	Round(..)
	Square(..)
	Curly(..)
	Bits(..)
	Quote(..)
	Unquote(..)
	Splice(..)
	TypeHandle
	Position
	ImportItem(..)
	ExportItem(..)
	Item(..)
	Attribute(..)
	Constructor(..)
	Field(..)
	Type(..)
	FunctionResult(..)
	Parameter(..)
	Expression(..)
	Statement(..)
	CallArgument(..)
	Clause(..)
	ClauseHead(..)
	Pattern(..)
	Segment(..)
	Format(..)
	SyntaxError(..)
	Definitions(..)
	definition
	node
	make
	same_code
	show
	show_as
	Items(..)
	Diagnostics(..)
	Types(..)
	Truth
	Yes(..)
	No(..)
	NotKnown(..)
	type_of
	signature
	subtype
	members
)

Syntax = Opaque

SyntaxList = Opaque

SyntaxResult = Opaque

TextResult = Opaque

MatchResult = Opaque

DefinitionResult = Opaque

ItemMetadata = Opaque

ItemResult = Opaque

TypeHandle = Opaque

/// A node of syntax. `Int` and `Float` are the built-in types; the other
/// members share a type constructor with another member, so each is its own
/// type (syntax three §2.8).
Node = (
	TypeView
		| Name
		| Discard
		| Up
		| Symbol
		| Int
		| Float
		| Text
		| Round
		| Square
		| Curly
		| Bits
		| Quote
		| Unquote
		| Splice,
)

TypeView = (TypeHandle)

Name = (String)

Discard = (String)

Up = (String)

Symbol = (String)

Text = (String)

Round = (List(Syntax))

Square = (List(Syntax))

Curly = (List(Syntax))

Bits = (List(Syntax))

Quote = (Syntax)

Unquote = (Syntax)

Splice = (Syntax)

Position = ImportItem
	| ExportItem
	| Item
	| Attribute
	| Constructor
	| Field
	| Type
	| FunctionResult
	| Parameter
	| Expression
	| Statement
	| CallArgument
	| Clause
	| ClauseHead
	| Pattern
	| Segment
	| Format

ImportItem = ()

ExportItem = ()

Item = ()

Attribute = ()

Constructor = ()

Field = ()

Type = ()

FunctionResult = ()

Parameter = ()

Expression = ()

Statement = ()

CallArgument = ()

Clause = ()

ClauseHead = ()

Pattern = ()

Segment = ()

Format = ()

SyntaxError = (message = String, at = Syntax)

Definitions = (definition = (Syntax) <- (Fail(SyntaxError)) -> Syntax)

/// Asks the installed definition service for a type's written definition.
definition = (of : Syntax) <- (service : Definitions) -> Syntax {
	service.definition(of)
}

Diagnostics = (note = (String) -> Nil)

@external("i:compiler/syntax/v1", "note")
raw_note : (String) -> Nil

/// Constructs the compiler's transactional diagnostic service.
_diagnostics = () -> Diagnostics {
	notify = (message : String) -> Nil { raw_note(message) }
	Diagnostics(note = notify)
}

Items = (
	add = (Syntax, Syntax, List((String, String)), Syntax) <- (
		Fail(SyntaxError),
	) -> Nil,
	expose = (Syntax) <- (Fail(SyntaxError)) -> Nil,
)

@external("i:compiler/syntax/v1", "metadata_new")
raw_metadata_new : () -> ItemMetadata

@external("i:compiler/syntax/v1", "metadata_push")
raw_metadata_push : (ItemMetadata, String, String) -> ItemMetadata

@external("i:compiler/syntax/v1", "items_add")
raw_items_add : (Syntax, Syntax, ItemMetadata, Syntax) -> ItemResult

@external("i:compiler/syntax/v1", "items_expose")
raw_items_expose : (Syntax) -> ItemResult

@external("i:compiler/syntax/v1", "items_ok")
raw_items_ok : (ItemResult) -> Bool

@external("i:compiler/syntax/v1", "items_message")
raw_items_message : (ItemResult) -> String

@external("i:compiler/syntax/v1", "items_at")
raw_items_at : (ItemResult) -> Syntax

build_metadata = (
	data : List((String, String)),
	result : ItemMetadata
) -> ItemMetadata {
	data |>
		([]) { result }
		| ([(key, value), ..rest]) {
			build_metadata(rest, raw_metadata_push(result, key, value))
		}
}

item_result = (result : ItemResult) -> Nil {
	raw_items_ok(result) |>
		(True) { Nil }
		| (False) {
			fail.raise(
				SyntaxError(
					message = raw_items_message(result),
					at = raw_items_at(result),
				),
			)
		}
}

/// Constructs the compiler's transactional item service for one macro run.
_items = () -> Items {
	Items(
		add = (collection, target, data, at) -> Nil {
			item_result(
				raw_items_add(
					collection,
					target,
					build_metadata(data, raw_metadata_new()),
					at,
				),
			)
		},
		expose = (name) -> Nil { item_result(raw_items_expose(name)) },
	)
}

@external("i:compiler/syntax/v1", "report_error")
_report_error : (String, Syntax) -> Syntax

@external("i:compiler/syntax/v1", "definition")
raw_definition : (Syntax) -> DefinitionResult

@external("i:compiler/syntax/v1", "definition_ok")
raw_definition_ok : (DefinitionResult) -> Bool

@external("i:compiler/syntax/v1", "definition_value")
raw_definition_value : (DefinitionResult) -> Syntax

@external("i:compiler/syntax/v1", "definition_message")
raw_definition_message : (DefinitionResult) -> String

@external("i:compiler/syntax/v1", "definition_at")
raw_definition_at : (DefinitionResult) -> Syntax

/// Constructs the compiler's definition service for one macro run.
_definitions = () -> Definitions {
	Definitions(
		definition = (of) -> Syntax {
			result = raw_definition(of)
			raw_definition_ok(result) |>
				(True) { raw_definition_value(result) }
				| (False) {
					fail.raise(
						SyntaxError(
							message = raw_definition_message(result),
							at = raw_definition_at(result),
						),
					)
				}
		},
	)
}

@external("i:compiler/syntax/v1", "match")
_match : (Syntax, Syntax) -> MatchResult

@external("i:compiler/syntax/v1", "match_ok")
_match_ok : (MatchResult) -> Bool

@external("i:compiler/syntax/v1", "match_one")
_match_one : (MatchResult, Int) -> Syntax

@external("i:compiler/syntax/v1", "match_many_len")
raw_match_many_len : (MatchResult, Int) -> Int

@external("i:compiler/syntax/v1", "match_many_child")
raw_match_many_child : (MatchResult, Int, Int) -> Syntax

@external("i:compiler/syntax/v1", "template")
_template : (Int) -> Syntax

@external("i:compiler/syntax/v1", "rebuild")
raw_rebuild : (Syntax, SyntaxList) -> Syntax

@external("i:compiler/syntax/v1", "kind")
raw_kind : (Syntax) -> Int

@external("i:compiler/syntax/v1", "text")
raw_text : (Syntax) -> String

@external("i:compiler/syntax/v1", "int")
raw_int : (Syntax) -> Int

@external("i:compiler/syntax/v1", "float")
raw_float : (Syntax) -> Float

@external("i:compiler/syntax/v1", "length")
raw_length : (Syntax) -> Int

@external("i:compiler/syntax/v1", "child")
raw_child : (Syntax, Int) -> Syntax

@external("i:compiler/syntax/v1", "list_new")
raw_list_new : () -> SyntaxList

@external("i:compiler/syntax/v1", "list_push")
raw_list_push : (SyntaxList, Syntax) -> SyntaxList

@external("i:compiler/syntax/v1", "make_text")
raw_make_text : (Int, String, Syntax) -> SyntaxResult

@external("i:compiler/syntax/v1", "make_int")
raw_make_int : (Int, Syntax) -> SyntaxResult

@external("i:compiler/syntax/v1", "make_float")
raw_make_float : (Float, Syntax) -> SyntaxResult

@external("i:compiler/syntax/v1", "make_list")
raw_make_list : (Int, SyntaxList, Syntax) -> SyntaxResult

@external("i:compiler/syntax/v1", "make_one")
raw_make_one : (Int, Syntax, Syntax) -> SyntaxResult

@external("i:compiler/syntax/v1", "syntax_ok")
raw_syntax_ok : (SyntaxResult) -> Bool

@external("i:compiler/syntax/v1", "syntax_value")
raw_syntax_value : (SyntaxResult) -> Syntax

@external("i:compiler/syntax/v1", "syntax_error")
raw_syntax_error : (SyntaxResult) -> String

@external("i:compiler/syntax/v1", "same_code")
raw_same_code : (Syntax, Syntax) -> Bool

@external("i:compiler/syntax/v1", "show")
raw_show : (Syntax) -> String

@external("i:compiler/syntax/v1", "show_as")
raw_show_as : (Syntax, Int) -> TextResult

@external("i:compiler/syntax/v1", "text_ok")
raw_text_ok : (TextResult) -> Bool

@external("i:compiler/syntax/v1", "text_value")
raw_text_value : (TextResult) -> String

@external("i:compiler/syntax/v1", "text_error")
raw_text_error : (TextResult) -> String

@external("i:compiler/syntax/v1", "text_error_at")
raw_text_error_at : (TextResult) -> Syntax

reverse = (items : List(Syntax), result : List(Syntax)) -> List(Syntax) {
	items |>
		([]) { result }
		| ([first, ..rest]) { reverse(rest, [first, ..result]) }
}

match_children = (
	match : MatchResult,
	capture : Int,
	index : Int,
	end : Int,
	result : List(Syntax)
) -> List(Syntax) {
	{ index == end } |>
		(True) { reverse(result, []) }
		| (False) {
			match_children(
				match,
				capture,
				index + 1,
				end,
				[raw_match_many_child(match, capture, index), ..result],
			)
		}
}

/// Gives a splice capture in source order.
_match_many = (match : MatchResult, capture : Int) -> List(Syntax) {
	match_children(match, capture, 0, raw_match_many_len(match, capture), [])
}

prepend_reversed = (items : List(Syntax), result : List(Syntax)) -> List(
	Syntax,
) {
	items |>
		([]) { result }
		| ([first, ..rest]) { prepend_reversed(rest, [first, ..result]) }
}

/// Appends syntax lists while preserving the identity and order of each item.
_append = (first : List(Syntax), second : List(Syntax)) -> List(Syntax) {
	prepend_reversed(reverse(first, []), second)
}

children = (
	syntax : Syntax,
	index : Int,
	end : Int,
	result : List(Syntax)
) -> List(Syntax) {
	{ index == end } |>
		(True) { reverse(result, []) }
		| (False) {
			children(
				syntax,
				index + 1,
				end,
				[raw_child(syntax, index), ..result],
			)
		}
}

/// Gives the node without changing the scopes of its children.
node = (syntax : Syntax) -> Node {
	raw_kind(syntax) |>
		(0) { Node(Name(raw_text(syntax))) }
		| (1) { Node(Discard(raw_text(syntax))) }
		| (2) { Node(Up(raw_text(syntax))) }
		| (3) { Node(Symbol(raw_text(syntax))) }
		| (4) { Node(raw_int(syntax)) }
		| (5) { Node(raw_float(syntax)) }
		| (6) { Node(Text(raw_text(syntax))) }
		| (7) { Node(Round(children(syntax, 0, raw_length(syntax), []))) }
		| (8) { Node(Square(children(syntax, 0, raw_length(syntax), []))) }
		| (9) { Node(Curly(children(syntax, 0, raw_length(syntax), []))) }
		| (10) { Node(Bits(children(syntax, 0, raw_length(syntax), []))) }
		| (11) { Node(Quote(raw_child(syntax, 0))) }
		| (12) { Node(Unquote(raw_child(syntax, 0))) }
		| (13) { Node(Splice(raw_child(syntax, 0))) }
		| (14) { Node(TypeView(raw_type_handle(syntax))) }
		| (_) { panic("syntax: invalid node kind") }
}

build_list = (items : List(Syntax), result : SyntaxList) -> SyntaxList {
	items |>
		([]) { result }
		| ([first, ..rest]) { build_list(rest, raw_list_push(result, first)) }
}

/// Rebuilds one quoted template node with its evaluated children.
_rebuild = (template : Syntax, children : List(Syntax)) -> Syntax {
	raw_rebuild(
		template,
		build_list(children, raw_list_new()),
	)
}

/// Makes syntax at the span and scope of `like`.
@external("i:compiler/syntax/v1", "type_handle")
raw_type_handle : (Syntax) -> TypeHandle

make = (node : Node, like : Syntax) -> Syntax {
	node |>
		(Node(TypeView(_))) {
			fail.raise(
				SyntaxError(
					message = "make cannot construct a TypeView",
					at = like,
				),
			)
		}
		| (_) { Nil }
	result = node |>
		(Node(Name(value))) { raw_make_text(0, value, like) }
		| (Node(Discard(value))) { raw_make_text(1, value, like) }
		| (Node(Up(value))) { raw_make_text(2, value, like) }
		| (Node(Symbol(value))) { raw_make_text(3, value, like) }
		| (Node(value: Int)) { raw_make_int(value, like) }
		| (Node(value: Float)) { raw_make_float(value, like) }
		| (Node(Text(value))) { raw_make_text(6, value, like) }
		| (Node(Round(items))) {
			raw_make_list(
				7,
				build_list(items, raw_list_new()),
				like,
			)
		}
		| (Node(Square(items))) {
			raw_make_list(
				8,
				build_list(items, raw_list_new()),
				like,
			)
		}
		| (Node(Curly(items))) {
			raw_make_list(
				9,
				build_list(items, raw_list_new()),
				like,
			)
		}
		| (Node(Bits(items))) {
			raw_make_list(
				10,
				build_list(items, raw_list_new()),
				like,
			)
		}
		| (Node(Quote(item))) { raw_make_one(11, item, like) }
		| (Node(Unquote(item))) { raw_make_one(12, item, like) }
		| (Node(Splice(item))) { raw_make_one(13, item, like) }
		| (Node(TypeView(_))) { panic("TypeView make invariant") }
	raw_syntax_ok(result) |>
		(True) { raw_syntax_value(result) }
		| (False) {
			fail.raise(
				SyntaxError(message = raw_syntax_error(result), at = like),
			)
		}
}

/// Compares datums without comparing source locations or scopes.
same_code = (a : Syntax, b : Syntax) -> Bool { raw_same_code(a, b) }

/// Writes a datum in Lisp form on one line.
show = (syntax : Syntax) -> String { raw_show(syntax) }

position_id = (position : Position) -> Int {
	position |>
		(ImportItem) { 0 }
		| (ExportItem) { 1 }
		| (Item) { 2 }
		| (Attribute) { 3 }
		| (Constructor) { 4 }
		| (Field) { 5 }
		| (Type) { 6 }
		| (FunctionResult) { 7 }
		| (Parameter) { 8 }
		| (Expression) { 9 }
		| (Statement) { 10 }
		| (CallArgument) { 11 }
		| (Clause) { 12 }
		| (ClauseHead) { 13 }
		| (Pattern) { 14 }
		| (Segment) { 15 }
		| (Format) { 16 }
}

/// Writes syntax in the normal surface at the requested position.
show_as = (syntax : Syntax, position : Position) -> String {
	result = raw_show_as(syntax, position_id(position))
	raw_text_ok(result) |>
		(True) { raw_text_value(result) }
		| (False) {
			fail.raise(
				SyntaxError(
					message = raw_text_error(result),
					at = raw_text_error_at(result),
				),
			)
		}
}

/// Frozen type observations supplied only to typed macro implementations.
Truth = Yes | No | NotKnown

Yes = ()

No = ()

NotKnown = ()

Types = (
	type_of = (Syntax) <- (Fail(SyntaxError)) -> Syntax,
	signature = () -> Syntax,
	subtype = (Syntax, Syntax) <- (Fail(SyntaxError)) -> Truth,
	members = (Syntax) <- (Fail(SyntaxError)) -> List(Syntax),
)

TypeResult = Opaque

@external("i:compiler/syntax/v1", "type_of")
raw_type_of : (Syntax) -> TypeResult

@external("i:compiler/syntax/v1", "signature")
raw_signature : () -> TypeResult

@external("i:compiler/syntax/v1", "subtype")
raw_subtype : (Syntax, Syntax) -> TypeResult

@external("i:compiler/syntax/v1", "members")
raw_members : (Syntax) -> TypeResult

@external("i:compiler/syntax/v1", "type_ok")
raw_type_ok : (TypeResult) -> Bool

@external("i:compiler/syntax/v1", "type_value")
raw_type_value : (TypeResult) -> Syntax

@external("i:compiler/syntax/v1", "type_truth")
raw_type_truth : (TypeResult) -> Int

@external("i:compiler/syntax/v1", "type_len")
raw_type_len : (TypeResult) -> Int

@external("i:compiler/syntax/v1", "type_child")
raw_type_child : (TypeResult, Int) -> Syntax

@external("i:compiler/syntax/v1", "type_message")
raw_type_message : (TypeResult) -> String

@external("i:compiler/syntax/v1", "type_at")
raw_type_at : (TypeResult) -> Syntax

type_result = (r : TypeResult) -> Syntax {
	raw_type_ok(r) |>
		(True) { raw_type_value(r) }
		| (False) {
			fail.raise(
				SyntaxError(message = raw_type_message(r), at = raw_type_at(r)),
			)
		}
}

type_members = (r : TypeResult, index : Int, count : Int) -> List(Syntax) {
	{ index >= count } |>
		(True) { [] }
		| (False) {
			[raw_type_child(r, index), ..type_members(r, index + 1, count)]
		}
}

_types = () -> Types {
	Types(
		type_of = (s) -> Syntax { type_result(raw_type_of(s)) },
		signature = () -> Syntax { raw_type_value(raw_signature()) },
		subtype = (a, b) -> Truth {
			r = raw_subtype(a, b)
			raw_type_ok(r) |>
				(False) {
					fail.raise(
						SyntaxError(
							message = raw_type_message(r),
							at = raw_type_at(r),
						),
					)
				}
				| (True) {
					raw_type_truth(r) |>
						(0) { Yes }
						| (1) { No }
						| (_) { NotKnown }
				}
		},
		members = (s) -> List(Syntax) {
			r = raw_members(s)
			raw_type_ok(r) |>
				(False) {
					fail.raise(
						SyntaxError(
							message = raw_type_message(r),
							at = raw_type_at(r),
						),
					)
				}
				| (True) { type_members(r, 0, raw_type_len(r)) }
		},
	)
}

type_of = (s : Syntax) <- (types : Types) -> Syntax { types.type_of(s) }

signature = () <- (types : Types) -> Syntax { types.signature() }

subtype = (a : Syntax, b : Syntax) <- (types : Types) -> Truth {
	types.subtype(a, b)
}

members = (s : Syntax) <- (types : Types) -> List(Syntax) { types.members(s) }
