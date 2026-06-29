(comment) @comment

(string) @string
(url_literal) @string.special
(escape_sequence) @string.escape
(formatted_string) @string
(formatted_string_content) @string
(formatted_escape_sequence) @string.escape
(interpolation
  expression: (identifier) @variable)
(number) @number
(boolean) @boolean
(null) @constant.builtin

(identifier) @variable

((identifier) @variable.special
 (#eq? @variable.special "it"))

(lambda_expression
  parameter: (identifier) @variable.parameter)

(function_declaration
  name: (identifier) @function)

(function_parameter_list
  (identifier) @variable.parameter)

(test_block
  "test" @keyword)

(test_input
  name: (identifier) @variable.parameter)

(call_expression
  function: (identifier) @function)

((call_expression
  function: (identifier) @function.builtin)
 (#any-of? @function.builtin "fetch" "extend"))

(method_call_expression
  method: (identifier) @function)

(functional_block_expression
  method: (identifier) @function)

(property_access_expression
  property: (identifier) @property)

(switch_arm
  pattern: (identifier) @variable.parameter)

(pair
  key: (identifier) @property)

(pair
  key: (string) @property)

[
  "="
  "+="
  "-="
  "=>"
  "!"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "+"
  "-"
  "*"
  "/"
  "%"
  "&&"
  "||"
  "and"
  "or"
  "not"
] @operator

[
  "if"
  "else"
  "switch"
  "fun"
  "return"
  "while"
  "loop"
] @keyword

(break_statement) @keyword

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  ":"
  "."
] @punctuation.delimiter

[
  "@"
] @punctuation.special
