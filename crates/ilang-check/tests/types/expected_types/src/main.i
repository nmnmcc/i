// §4.8: expected types give known types to the parameters of function
// expressions that have no written type, as in TypeScript.
(
	app/list
)

User = (name = String, age = Int)

users: List(User) = [User(name = "ann", age = 30)]

// An argument of a call whose function has a known function type: the type
// variables of its parameter come from the other arguments.
names = list.map(users, (u) { u.name })

// The call has the known type `List(String)`.
first_name = () {
	list.map(users, (u) { u.name }) |> ([n, ..]) { n == 1 } | ([]) { False }
}

// Also with `|>`, with labels, and with more than one type variable.
piped = users |> list.map((u) { u.age })

by_label = list.map(users, (u) { u.name })

total = list.fold(users, 0, (sum, u) { sum + u.age })

// Without a known type for `users`, `u` has none.
unknown = (people) { list.map(people, (u) { u.name }) }

// The value of a binding with `x: T` at the top of its pattern.
annotated: (User) -> String = (u) { u.name }

// The body of a function with a written `->` part.
getter = () -> (User) -> String { (u) { u.name } }

// The last expression of a block, and the clause expressions of a match.
in_block: (User) -> Int = {
	n = 1
	(u) { u.age + n }
}

in_case = (b: Bool) -> (User) -> String {
	b |> (True) { (u) { u.name } } | (False) { (u) { "" } }
}

// The parts of a tuple and of a list.
in_tuple: ((User) -> String, Int) = ((u) { u.name }, 1)

in_list: List((User) -> Int) = [(u) { u.age }, (v) { v.age + 1 }]

// A function expression with a different number of parameters gets none.
arity: (User) -> String = (u, v) { u.name }

// With no expected type, `.f` needs a written type.
alone = (u) { u.name }
