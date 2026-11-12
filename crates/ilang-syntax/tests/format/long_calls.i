short = () { f(a, b, c) }

long = () {
	result = some_module.some_function(
		first_argument,
		second_argument,
		third_argument,
		fourth,
	)
	nested = outer(
		inner_function(alpha_value, beta_value, gamma_value),
		another_inner(delta, epsilon_value),
	)
	labels = make_user(
		name = "a long name for a user",
		age = 42,
		email_address = "someone@example.com",
	)
	record = User(
		..record,
		name = "another long name for the user",
		age = 43,
		email = "x@y.z",
	)
	list = [
		first_element_of_list,
		second_element_of_list,
		third_element_of_list,
		fourth,
	]
	tuple = (
		first_element_of_tuple,
		second_element_of_tuple,
		third_element_of_tuple,
		f,
	)
	bits = <<
		first_segment_value:u8,
		second_segment_value:u16be,
		third_segment_value:bits,
	>>
	hugged = list.map(items, (item) {
		doubled = item * 2
		doubled + 1
	})
	list
}
