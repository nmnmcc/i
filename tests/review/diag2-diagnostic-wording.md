# Diagnostic wording changes

Baseline: `4bbfc4b7b`, with main `a02d34100` merged.

This is a reporting change only. The original inventory at `982ba1524` has 136 Rust literals and 169 section marks: 116 message literals / 128 marks; nine test/assertion literals / 11 marks (seven expected-message marks and four ignored-test-description marks); 11 embedded program or package-template literals / 30 marks. The merged baseline has 154 literals and 189 marks: 134 message literals / 146 marks; nine test/assertion literals / 11 marks (the same seven expected-message marks and four ignored-test-description marks); 11 embedded programs/templates / 32 marks.

Of the 30 original embedded marks, one is in an i test input comment, one is in the generated package-template comment, and 28 are in WAT runtime/emitter comments. These are not compiler diagnostic messages. Those programs and templates remain exact. Four test-ignore descriptions are also retained; the other five assertion literals now test the same language rules in plain words.

All source message occurrences below are counted, including the legacy resolver and migration tools. They are potential diagnostic emitters, even when their parser path is dormant.

| Message class before | Message class after | Occurrences |
| --- | --- | ---: |
| an ordinary entry cannot reach a compiler-owned operation (§2.1) | an ordinary entry cannot reach a compiler-owned operation | 2 |
| an ordinary entry's initialization cannot reach a compiler-owned operation (§2.1) | an ordinary entry's initialization cannot reach a compiler-owned operation | 2 |
| a macro program cannot execute an external declaration outside package i (§8) | a macro program cannot execute an external declaration outside package i | 1 |
| a compiler wrapper must install its allowed services and cannot request a host service (§8) | a compiler wrapper must install its allowed services and cannot request a host service | 1 |
| the value of {name} is needed to work out itself (§2.5); only names inside the function bodies of a direct value may refer back | the value of {name} is needed to work out itself; only names inside the function bodies of a direct value may refer back | 1 |
| &#96;{service}&#96; is not provided by the target &#96;{}&#96;: the entry &#96;{}.{}&#96; needs it, so give it in the entry with i code (§2.10.11) | &#96;{service}&#96; is not provided by the target &#96;{}&#96;: the entry &#96;{}.{}&#96; needs it, so give it in the entry with i code | 1 |
| marked &#96;{name}&#96; conflicts with a name in its scope module (§5) | marked &#96;{name}&#96; conflicts with a name in its scope module | 2 |
| marked &#96;{name}&#96; cannot be exported (§5) | marked &#96;{name}&#96; cannot be exported | 2 |
| &#96;{first}&#96; is not &#96;i&#96;, the name of this package, or a dependency name (§2.1) | &#96;{first}&#96; is not &#96;i&#96;, the name of this package, or a dependency name | 2 |
| &#96;{}&#96; is a local value here, not a module; a local name covers a module name (§2.3) | &#96;{}&#96; is a local value here, not a module; a local name covers a module name | 2 |
| the label &#96;{name}&#96; is not a name; a label of data is a &#96;name&#96; (syntax-3 §2.3) | the label &#96;{name}&#96; is not a name; a label of data is a &#96;name&#96; | 2 |
| a service type is one named type &#96;T(…)&#96;: not a type variable, a union, or &#96;Never&#96; (§9.2) | a service type is one named type &#96;T(…)&#96;: not a type variable, a union, or &#96;Never&#96; | 2 |
| a host field has ABI parameter, result and error types, and no written environment; request fields return Request(A) or Request(Result(A,E)), with no written errors (§9.7, §10.3){input_note} | a host field's parameters and result must use &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, &#96;BitArray&#96;, or opaque types with no constructors. A field may have no need or exactly &#96;Fail(E)&#96;, where &#96;E&#96; contains only the six built-in types. Request fields return &#96;Request(A)&#96; or &#96;Request(Result(A, E))&#96;, with no written need or errors{input_note} | 2 |
| the type of an external function is a function type (§10.1) | the type of an external function is a function type | 2 |
| an external function writes errors as a Fail(E) need (§10.1) | an external function writes errors as a Fail(E) need | 2 |
| the full type of an external function must be written, with no &#96;_&#96; (§10.1) | the full type of an external function must be written, with no &#96;_&#96; | 4 |
| the type of an external function may not have type variables (§10.1) | the type of an external function may not have type variables | 4 |
| the error of an external function must be a host-passable error type (§10.3) | the error of an external function must be &#96;Never&#96; or a union of &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, and &#96;BitArray&#96;; opaque types cannot be errors | 2 |
| an external function may have no need or exactly Fail(E) (§10.1) | an external function may have no need or exactly Fail(E) | 2 |
| the parameters and the value of an external function are types that the host can take: &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, &#96;BitArray&#96;, or a custom type with no constructors (§10.3){} | the parameters and the value of an external function are types that the host can take: &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, &#96;BitArray&#96;, or a custom type with no constructors{} | 2 |
| &#96;{name}&#96; cannot bind a name with this macro origin (§5) | &#96;{name}&#96; cannot bind a name with this macro origin | 2 |
| macro &#96;{}&#96; can only be used as a macro call (§4.1) | macro &#96;{}&#96; can only be used as a macro call | 4 |
| {what} is exported, but its type uses the type &#96;{name}&#96; {}, which is not exported (§2.4) | {what} is exported, but its type uses the type &#96;{name}&#96; {}, which is not exported | 2 |
| a custom type has one constructor or none; write one type for each constructor and name their union: &#96;{} = {}&#96; (§4.3) | a custom type has one constructor or none; write one type for each constructor and name their union: &#96;{} = {}&#96; | 1 |
| a host service has exactly one constructor, and every field has a label and a function type (§9.7) | a host service has exactly one constructor, and every field has a label and a function type | 2 |
| a data type has no spread: records are closed (syntax-3 §2.3) | a data type has no spread: records are closed | 1 |
| a data type has no forgotten labels: records are closed (syntax-3 §2.3) | a data type has no forgotten labels: records are closed | 1 |
| a rest service pattern needs a type parameter of the function (§6.8) | a rest service pattern needs a type parameter of the function | 2 |
| two environment parameters of this function have the type constructor &#96;{tname}&#96; (§9.2) | two environment parameters of this function have the type constructor &#96;{tname}&#96; | 2 |
| the items of a list get the whole type &#96;{whole}&#96; from literal patterns (§4.8) | the items of a list get the whole type &#96;{whole}&#96; from literal patterns | 1 |
| no branch of this function can match these arguments: their known types are &#96;({})&#96; (syntax three §2.6.2) | no branch of this function can match these arguments: their known types are &#96;({})&#96; | 1 |
| a top-level binding may not need an environment (§2.5), but this one has &#96;{}&#96; | a top-level binding may not need an environment, but this one has &#96;{}&#96; | 1 |
| this pattern may fail, so it needs &#96;Assert&#96; (§7.3), but &#96;Assert&#96; is not allowed here: &#96;Assert&#96; is not a subtype of &#96;{}&#96; | this pattern may fail, so it needs &#96;Assert&#96;, but &#96;Assert&#96; is not allowed here: &#96;Assert&#96; is not a subtype of &#96;{}&#96; | 1 |
| a result-only signature variable has no lower evidence, so the output cannot choose its type (§15.7.5): {} | a result-only signature variable has no lower evidence, so the output cannot choose its type: {} | 1 |
| the output does not have the declared result type; a call's type cannot depend on its expansion (§15.7.5): {} | the output does not have the declared result type; a call's type cannot depend on its expansion: {} | 1 |
| entry &#96;{name}&#96; must be a function value (§10.2) | entry &#96;{name}&#96; must be a function value | 1 |
| entry &#96;{name}&#96; has no parameters but environment parameters (§10.2) | entry &#96;{name}&#96; has no parameters but environment parameters | 1 |
| &#96;{name}&#96; is a host export, so it is a function (§10.2), but its type is &#96;{t}&#96; | &#96;{name}&#96; is a host export, so it is a function, but its type is &#96;{t}&#96; | 1 |
| &#96;{name}&#96; is a host export, so {what} must be a type that the host can take: &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, &#96;BitArray&#96;, or a custom type with no constructors (§10.3), but it is &#96;{t}&#96;{suffix} | &#96;{name}&#96; is a host export, so {what} must be a type that the host can take: &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, &#96;BitArray&#96;, or a custom type with no constructors, but it is &#96;{t}&#96;{suffix} | 1 |
| &#96;{name}&#96; is a host export, so its errors must be a union of &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, and &#96;BitArray&#96; (§10.3), but they are &#96;{e}&#96; | &#96;{name}&#96; is a host export, so its errors must be a union of &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, and &#96;BitArray&#96;, but they are &#96;{e}&#96; | 1 |
| &#96;{}&#96; is exported, but its inferred type uses the type &#96;{name}&#96;{location}, which is not exported (§2.4) | &#96;{}&#96; is exported, but its inferred type uses the type &#96;{name}&#96;{location}, which is not exported | 1 |
| a union with two or more type variables as members may only be in an output place (§4.4): inference could not choose the variable for a value that comes here | a union with two or more type variables as members may only be in an output place: inference could not choose the variable for a value that comes here | 1 |
| an intersection with a type variable may only be in an input place (§4.4): inference could not give a value that is in both | an intersection with a type variable may only be in an input place: inference could not give a value that is in both | 1 |
| this type has itself inside it, through a type variable (§4.7) | this type has itself inside it, through a type variable | 2 |
| &#96;{l}&#96; and &#96;{r}&#96; must be equal: they are arguments of two forms of one type constructor whose parameter does not change, which are put together (§4.4) | &#96;{l}&#96; and &#96;{r}&#96; must be equal: they are arguments of two forms of one type constructor whose parameter does not change, which are put together | 1 |
| the service requirements &#96;{l}&#96; and &#96;{r}&#96; have an empty intersection and cannot be satisfied (§4.4, §9.2) | the service requirements &#96;{l}&#96; and &#96;{r}&#96; have an empty intersection and cannot be satisfied | 1 |
| &#96;{l}&#96; and &#96;{r}&#96; do not agree: a service in an environment meets a need when it is a subtype of the needed form as a value, and two forms of one type constructor are one entry, so arguments whose parameter does not change must be equal (§4.5 rule 4, §9.2) | &#96;{l}&#96; and &#96;{r}&#96; do not agree: a service in an environment meets a need when it is a subtype of the needed form as a value, and two forms of one type constructor are one entry, so arguments whose parameter does not change must be equal | 1 |
| a value of &#96;{l}&#96; could go to more than one type variable of &#96;{r}&#96; (§4.4) | a value of &#96;{l}&#96; could go to more than one type variable of &#96;{r}&#96; | 1 |
| the right side of &#96;-&#96; may only have members with a type constructor (§4.4) | the right side of &#96;-&#96; may only have members with a type constructor | 1 |
| an environment row may only be bound, captured or provided after &#96;<-&#96;; it is not ordinary data (§9.4) | an environment row may only be bound, captured or provided after &#96;<-&#96;; it is not ordinary data | 1 |
| a spread &#96;..e&#96; needs a known type for &#96;e&#96; with exactly one member, which is data (§2.3), but it has &#96;{k}&#96; | a spread &#96;..e&#96; needs a known type for &#96;e&#96; with exactly one member, which is data, but it has &#96;{k}&#96; | 1 |
| both sides are constants, &#96;{l}&#96; and &#96;{r}&#96;, which can never be equal, so &#96;{operator}&#96; is always &#96;{result}&#96; (§6.9) | both sides are constants, &#96;{l}&#96; and &#96;{r}&#96;, which can never be equal, so &#96;{operator}&#96; is always &#96;{result}&#96; | 1 |
| the values &#96;{l}&#96; and &#96;{r}&#96; can never be equal, so &#96;{operator}&#96; is always &#96;{result}&#96; (§6.9) | the values &#96;{l}&#96; and &#96;{r}&#96; can never be equal, so &#96;{operator}&#96; is always &#96;{result}&#96; | 1 |
| the two sides have the known types &#96;{l}&#96; and &#96;{r}&#96;, which can never be equal, so &#96;{operator}&#96; is always &#96;{result}&#96; (§6.9) | the two sides have the known types &#96;{l}&#96; and &#96;{r}&#96;, which can never be equal, so &#96;{operator}&#96; is always &#96;{result}&#96; | 1 |
| an argument by label may only be given to a constructor; give the arguments of a function by position (§6.6) | an argument by label may only be given to a constructor; give the arguments of a function by position | 1 |
| a call with an argument by label needs a known function type for the function (§6.6, §4.8); write the type of the function, or give the arguments by position | a call with an argument by label needs a known function type for the function; write the type of the function, or give the arguments by position | 1 |
| every parameter must get one argument (§6.6), but {} get{} none | every parameter must get one argument, but {} get{} none | 1 |
| &#96;.{}&#96; needs a known type for its value (§6.7, §4.8); write the parameter's type, for example &#96;(u : User) {{ u.name }}&#96;, or give the function where a type is expected, for example &#96;get_name : (User) -> String = (u) {{ u.name }}&#96;. An inferred type is solved from constraints; a known type comes from written types and earlier known facts, including the expected type, before solving (§4.8) | &#96;.{}&#96; needs a known type for its value; write the parameter's type, for example &#96;(u : User) {{ u.name }}&#96;, or give the function where a type is expected, for example &#96;get_name : (User) -> String = (u) {{ u.name }}&#96;. An inferred type is solved from constraints; a known type comes from written types and earlier known facts, including the expected type, before solving | 1 |
| &#96;.{}&#96; needs every member of the known type &#96;{k}&#96; of its value to be a custom type (§6.7), but &#96;{m}&#96; is not one | &#96;.{}&#96; needs every member of the known type &#96;{k}&#96; of its value to be a custom type, but &#96;{m}&#96; is not one | 1 |
| cannot select field &#96;{}&#96;: the constructors of &#96;{}&#96; are not exported from &#96;{}&#96; (§2.4, §6.7) | cannot select field &#96;{}&#96;: the constructors of &#96;{}&#96; are not exported from &#96;{}&#96; | 1 |
| the constructor &#96;{}&#96;, which made this value (§7.5), has no field with the label &#96;{}&#96; (§4.8 rule 13) | the constructor &#96;{}&#96;, which made this value, has no field with the label &#96;{}&#96; | 1 |
| not every constructor of &#96;{}&#96; has a field with the label &#96;{}&#96; (§4.8 rule 13) | not every constructor of &#96;{}&#96; has a field with the label &#96;{}&#96; | 1 |
| &#96;.k&#96; needs a known type for its value (§4.8 rule 13); write its type, such as &#96;x: (A, B)&#96; | &#96;.k&#96; needs a known type for its value; write its type, such as &#96;x: (A, B)&#96; | 1 |
| &#96;.{key_text}&#96; needs every member of the known type &#96;{whole}&#96; of its value to be data (§4.8 rule 13), but &#96;{m}&#96; is not | &#96;.{key_text}&#96; needs every member of the known type &#96;{whole}&#96; of its value to be data, but &#96;{m}&#96; is not | 1 |
| the data &#96;{m}&#96; has no part &#96;{key_text}&#96; (§2.3) | the data &#96;{m}&#96; has no part &#96;{key_text}&#96; | 1 |
| &#96;.{key_text}&#96; needs a known type for its value (§4.8 rule 13) | &#96;.{key_text}&#96; needs a known type for its value | 1 |
| &#96;{cname}(..v, …)&#96; needs &#96;v&#96; to be known to be made by &#96;{cname}&#96; (§6.7): its type has other constructors, so &#96;v&#96; must be a name matched with &#96;{cname}(…)&#96; (§7.5) | &#96;{cname}(..v, …)&#96; needs &#96;v&#96; to be known to be made by &#96;{cname}&#96;: its type has other constructors, so &#96;v&#96; must be a name matched with &#96;{cname}(…)&#96; | 1 |
| there is no bit array format &#96;{name}&#96; (§8.2) | there is no bit array format &#96;{name}&#96; | 1 |
| the format &#96;{name}&#96; needs a length: &#96;{name}(n)&#96; (§8.2) | the format &#96;{name}&#96; needs a length: &#96;{name}(n)&#96; | 1 |
| the format &#96;{name}&#96; has no length (§8.2) | the format &#96;{name}&#96; has no length | 1 |
| an opaque callback need must use the captured row parameter when other keys are excluded; write that row and its explicit service keys (§9.4) | an opaque callback need must use the captured row parameter when other keys are excluded; write that row and its explicit service keys | 1 |
| provided data has two parts with one service key (§9.2) | provided data has two parts with one service key | 1 |
| each provided part must have one known service type (§9.2) | each provided part must have one known service type | 1 |
| a whole provide needs positional data or a captured environment row (§9.2) | a whole provide needs positional data or a captured environment row | 1 |
| a provided service is keyed by its type, and the type of this value is not known to be one service type: give it a written type (syntax-3 §2.10.2) | a provided service is keyed by its type, and the type of this value is not known to be one service type: give it a written type | 1 |
| two explicit services or two spreads in provided data have one service key (§9.2) | two explicit services or two spreads in provided data have one service key | 1 |
| this pattern never matches: no value of the known type of what it is fitted to matches it (§4.8) | this pattern never matches: no value of the known type of what it is fitted to matches it | 1 |
| the type hides the environment statement's obligation for &#96;{}&#96;: every instance must use the same service form (§4.8 rule 9, §9.2) | the type hides the environment statement's obligation for &#96;{}&#96;: every instance must use the same service form | 1 |
| this pattern never matches: no value of this type is made by &#96;{name}&#96; (§4.8) | this pattern never matches: no value of this type is made by &#96;{name}&#96; | 1 |
| the format &#96;{name}&#96; in a pattern needs a string literal, which gives the bits to match (§8.4) | the format &#96;{name}&#96; in a pattern needs a string literal, which gives the bits to match | 1 |
| &#96;{name}&#96; with no length takes all the bits that are left, so it may only be the last segment (§8.4) | &#96;{name}&#96; with no length takes all the bits that are left, so it may only be the last segment | 1 |
| the constructor has no field with the label &#96;{l}&#96; (§7.1) | the constructor has no field with the label &#96;{l}&#96; | 1 |
| this part is matched by label, so it needs a name at its top that is the label of a field (§7.1) | this part is matched by label, so it needs a name at its top that is the label of a field | 1 |
| this field is matched more than once (§7.1) | this field is matched more than once | 1 |
| the constructor has {} field{}, and every one must be matched, or &#96;..&#96; must leave the rest out (§7.1) | the constructor has {} field{}, and every one must be matched, or &#96;..&#96; must leave the rest out | 1 |
| this clause is never used: every value that its pattern matches is matched by an earlier clause without a guard (§7.4) | this clause is never used: every value that its pattern matches is matched by an earlier clause without a guard | 1 |
| the clauses do not cover every value of the subject (§7.4): no clause without a guard matches &#96;{text}&#96; | the clauses do not cover every value of the subject: no clause without a guard matches &#96;{text}&#96; | 1 |
| in a test, &#96;x: T&#96; tests the type constructor, so &#96;T&#96; may not have a type variable as a member (§7.1) | in a test, &#96;x: T&#96; tests the type constructor, so &#96;T&#96; may not have a type variable as a member | 1 |
| the whole expanded value-type annotation has a variable union or intersection in an illegal place (§4.4) | the expanded value-type annotation puts type variables in an unsupported place: a union of two or more different type variables is allowed only in an output position, and an intersection with a type variable only in an input position | 1 |
| the package name &#96;{name}&#96; must be a name (§1.3): a lowercase letter, then lowercase letters, digits and &#96;_&#96;; and it may not be &#96;i&#96; | the package name &#96;{name}&#96; must be a name: a lowercase letter, then lowercase letters, digits and &#96;_&#96;; and it may not be &#96;i&#96; | 1 |
| the dependency name &#96;{name}&#96; must be a name (§1.3), not &#96;i&#96;, and not the name of this package (§2.6) | the dependency name &#96;{name}&#96; must be a name, not &#96;i&#96;, and not the name of this package | 1 |
| &#96;{name}&#96; is already in &#96;[{}]&#96;; a dependency name and a dev-dependency name may not be the same (§2.6) | &#96;{name}&#96; is already in &#96;[{}]&#96;; a dependency name and a dev-dependency name may not be the same | 1 |
| &#96;{}&#96; has no tag &#96;vX.Y.Z&#96;, so it has no version (§2.7) | &#96;{}&#96; has no tag &#96;vX.Y.Z&#96;, so it has no version | 1 |
| the package has no program entry; define &#96;main&#96; in &#96;src/main.i&#96; or run or build a declared entry by name (§10.2) | the package has no program entry; define &#96;main&#96; in &#96;src/main.i&#96; or run or build a declared entry by name | 1 |
| the WASI target runs a command entry and does not compile tests (§B.7) | the WASI target runs a command entry and does not compile tests | 1 |
| a typed macro cannot make items; use a syntax macro (§15.7.5) | a typed macro cannot make items; use a syntax macro | 1 |
| a typed macro cannot choose by the type inside a binding with type variables; a type query answered NotKnown here (§15.7.5) | a typed macro cannot choose by the type inside a binding with type variables; a type query answered NotKnown here | 1 |
| the label &#96;{}&#96; is not a name; syntax three has no string labels | the label &#96;{}&#96; is not a name; use a name rather than a string as a label | 1 |
| the type parameter &#96;{}&#96; of &#96;{}&#96; is lowercase: the old compiler rejected it (E0232) and syntax three cannot parse the head (E0101); rename it by hand or keep the file as an error fixture | the type parameter &#96;{}&#96; of &#96;{}&#96; is lowercase: type parameters need uppercase names; rename it by hand or keep the file as an error fixture | 1 |
| a record type with a spread or a removed label is an open record; data is closed in syntax three | a record type with a spread or a removed label is an open record; data types cannot have a spread or a removed label | 1 |
| a provided service has no label: it is keyed by its type (§2.10.2) | a provided service has no label: it is keyed by its type | 2 |
| type parameters on a function literal that is not the value of a binding need stage 7 of syntax three, which this compiler does not have yet | a function literal with type parameters must be the value of a binding | 2 |
| data with labels has only labelled items after an optional first &#96;..base&#96; (mixed data needs stage 6 of syntax three) | data with labels has only labelled items after an optional first &#96;..base&#96;; mixed positional and labelled items are not supported here | 2 |
| a named rest &#96;..name&#96; in a pattern needs stage 6 of syntax three, which this compiler does not have yet | a named rest &#96;..name&#96; in a pattern is not supported here | 2 |
| a key in a data pattern needs stage 6 of syntax three, which this compiler does not have yet | a key in a data pattern is not supported here | 2 |
| a rest &#96;..&#96; in a data pattern needs stage 6 of syntax three, which this compiler does not have yet | a rest &#96;..&#96; in a data pattern is not supported here | 2 |
| two patterns joined by &#96;=&#96; that are not names need stage 9 of syntax three, which this compiler does not have yet | two patterns joined by &#96;=&#96; must both be names here | 2 |
| a pattern inside a labelled field needs stage 4 of syntax three, which this compiler does not have yet; bind the field to a name and match the name | a pattern inside a labelled field is not supported here; bind the field to a name and match the name | 2 |
| a need has no label: it is keyed by its type (§2.10.2) | a need has no label: it is keyed by its type | 2 |
| type parameters on a function type that is not the type of a binding need stage 7 of syntax three, which this compiler does not have yet | a function type with type parameters must be the type of a binding | 2 |
| a data type with labels has only labelled items (mixed data needs stage 6 of syntax three) | a data type with labels has only labelled items; mixed positional and labelled items are not supported here | 2 |
| a key on a parameter needs shape groups (stage 9 of syntax three), which this compiler does not have yet | a key on a function parameter is not supported here | 1 |
| a chain whose branches have different shapes needs shape groups (stage 9 of syntax three), which this compiler does not have yet | a chain whose branches have different argument shapes is not supported here | 1 |
| this is one binding, not two; to write two, put the second on its own line (§1.7) | this is one binding, not two; to write two, put the second on its own line | 1 |
| synchronous export &#96;{}&#96; needs an asynchronous platform service (§2.6.2) | synchronous export &#96;{}&#96; needs an asynchronous platform service | 1 |
| the type of this external function is not one that the host can take (§10.1) | an external function needs a fully written function type with positional parameters. Parameters and results may use only &#96;Nil&#96;, &#96;Bool&#96;, &#96;Int&#96;, &#96;Float&#96;, &#96;String&#96;, &#96;BitArray&#96;, or opaque types. Its only allowed need is &#96;Fail(E)&#96;, where &#96;E&#96; contains only the six built-in types | 1 |
| a string format in a pattern needs a string literal (§8.4) | a string format in a pattern needs a string literal | 1 |
| a call with an argument by label of a function whose parameters the text does not show: the labels are kept as labelled items; the stage 4 migrator (&#96;labels&#96;) makes the call positional | this call's parameter order is not visible in the source, so its labels were kept. Write the arguments by position once you know the function's parameter order | 1 |

118 source message classes; 158 source occurrences. Exact Rust literals and locations are in `tests/review/diag2-diagnostic-wording.json`. The table shows source format templates, including escaped format braces; the corpus proof records rendered messages and their occurrence counts.

## Call reporting

The supervisor's reporting instruction is: “One error, one primary diagnostic.”
The design note's direct-call rule says: “**No selection** is a compile error
naming the known argument types.” Both requirements are preserved: the call is
still rejected, all of its original solver constraints still run, and E0501
reports the rejected argument's type at that argument's original position.
E0552 is dropped only when that same call already has an argument E0501.
When no individual argument mismatch explains failed selection, E0552 remains.
An independent coverage error in a definition (E0516) also remains.

For the supervisor's `bad = app.word(2)` input, before:

```text
src/main.i:2:7: error[E0552]: no branch of this function can match these arguments: their known types are `(2)` (syntax three §2.6.2)
src/main.i:2:16: error[E0501]: `2` is not a subtype of `0 | 1` (the type at 2:7 needs it)
2 errors
```

After:

```text
src/main.i:2:16: error[E0501]: `2` is not one of the values `app.word` accepts for argument 1: `0 | 1`
1 error
```

The two new message classes are:

- Finite literal argument: `` `{given}` is not one of the values `{name}` accepts for argument {position}: `{accepted}` ``.
- Other type: `` `{name}` does not accept argument {position} with type `{given}`; this argument must be `{accepted}` ``.

Both replace the pair E0552 “no branch of this function can match these
arguments: their known types are …” and E0501 “`given` is not a subtype of
`accepted` (the type at … needs it)”. Any grouping advice is retained after the
new message. E0501 is retained because it already identifies the exact argument
and its unsatisfied parameter constraint. The CLI's error-count footer follows
the remaining diagnostic count.

The new CLI tests cover integers, strings, a second argument, a piped argument,
a Boolean, one-item data, an independent binding error inside an argument, and
two independently bad arguments. A separate test
retains E0552 for a bad combination and its independent E0516 coverage error.
A matching literal call remains accepted. The renderer sorts by source position and error code, so the piped case prints
E0501 first. Deduplication only considers errors produced by the call's own
argument constraints; unrelated errors inside an argument retain their original
reporting.

The original 8 archived inputs remain byte-for-byte unchanged. Their exact paths
and SHA-256 values are in `tests/review/diag2-diagnostic-wording.json`. The
supervisor's `app.word(2)` example has no matching program in that archive; its
new CLI test keeps that expression verbatim and supplies its module.

## Corpus proof

`tests/review/diag2-citation-cases.json` lists the 762 records whose 3,289
diagnostic message occurrences lose references. Across the whole lane,
`tests/review/diag2-final-cases.json` lists all 771 records differing from main
`a02d34100`: 3,302 changed diagnostic message occurrences. The exact before and
after text of every change is in `tests/review/diag2-final-corpus-proof.json`.
All codes, positions, diagnostic counts, stdout, exit statuses, phase commands,
bounds and observation metadata remain exact. Existing corpus records contain
no E0552 pair; the new CLI tests and isolated inputs pin the duplicate counts.
These are reviewed wording updates, not fabricated compiler observations.
Independent compiler rerun results are recorded separately.

The host-field, external-error and Wasm fallback messages now enumerate allowed host types; the expanded-annotation message states the input/output rule directly. The former host-field text incorrectly said no written environment at all; its replacement describes the already-supported optional `Fail(E)` need and the request-mode restriction. These are wording corrections following the current host-service and host-boundary rules, with no checker condition changed.
