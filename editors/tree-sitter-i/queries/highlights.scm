; Keep the same classes as editors/vscode/syntaxes/i.tmLanguage.json.
(identifier) @variable
(type_identifier) @type
(discard) @variable.special
(integer) @number
(float) @number
(string) @string
(escape_sequence) @string.escape
(comment) @comment
(doc_comment) @comment.doc
(module_comment) @comment.doc
(attribute) @attribute
(bit_format) @type
(record_field label: (_) @variable.parameter)
(type_field label: (_) @variable.parameter)
(argument label: (identifier) @variable.parameter)
(typed_pattern pattern: (identifier) @variable.parameter)
(type_parameter (identifier) @variable.parameter)
(field_access field: (identifier) @property)
(call function: (identifier) @function)
(call function: (field_access field: (identifier) @function))
(binding pattern: (identifier) @function value: (function))
(binding pattern: (identifier) @function value: (clause_block))
(type_parameters (type_identifier) @type)
(module_path) @type
(import alias: (identifier) @type)
((type_identifier) @constant.builtin
 (#match? @constant.builtin "^(True|False|Nil)$"))
((type_identifier) @type.builtin
 (#match? @type.builtin "^(Record|Int|Float|String|List|BitArray|UtfCodepoint|Never|Bool|Result|Boundary|Continuation|Assert|AssertionFailure)$"))
["=" "->" "^" "?" "!" "|" "&" ".." "|>" "||" "&&" "==" "!=" "<" "<=" ">" ">=" "<." "<=." ">." ">=." "<>" "+" "-" "+." "-." "*" "/" "%" "*." "/."] @operator
["(" ")" "[" "]" "{" "}" "<<" ">>"] @punctuation.bracket
["," ":" "."] @punctuation.delimiter
