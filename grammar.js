/**
 * tree-sitter grammar for the Klass DSL.
 *
 * Ported from the authoritative ANTLR4 grammar:
 *   klass-model-converters/klass-grammar/src/main/antlr4/cool/klass/model/meta/grammar/Klass.g4
 *   klass-model-converters/klass-grammar/src/main/antlr4/imports/KlassLexerRules.g4
 *
 * Notes on the port:
 *  - ANTLR left-recursive rules (packageName, criteriaExpression) are rewritten
 *    using tree-sitter prec.left, since tree-sitter is GLR and does not allow
 *    the ANTLR flavor of direct left recursion in the same shape.
 *  - The ANTLR "notifyErrorListeners" alternatives (missing-semicolon recovery)
 *    are error-recovery productions; a formatter only ever sees well-formed input,
 *    so they are not modeled. The happy-path alternative is kept.
 *  - Comments (// line and /* block *\/) are declared as `extras` so they survive
 *    in the tree for the printer.
 */

module.exports = grammar({
	name: 'klass',

	extras: ($) => [/[\s]/, $.line_comment, $.block_comment],

	word: ($) => $.identifier_token,

	conflicts: ($) => [
		// A '/' after url path segments can either close the url (trailing slash)
		// or begin another segment; GLR resolves this with one token of lookahead.
		[$.url, $.url_path_segment],
	],

	rules: {
		compilation_unit: ($) => seq($.package_declaration, repeat($.top_level_declaration)),

		package_declaration: ($) => seq('package', $.package_name),

		// ANTLR: packageName '.' identifier  (left recursive) -> dotted path
		package_name: ($) => sep1($.identifier, '.'),

		top_level_declaration: ($) =>
			choice(
				$.interface_declaration,
				$.class_declaration,
				$.enumeration_declaration,
				$.association_declaration,
				$.projection_declaration,
				$.service_group_declaration,
			),

		// ---- interface ----
		interface_declaration: ($) => seq($.interface_header, $.interface_block),
		interface_header: ($) =>
			seq('interface', $.identifier, optional($.implements_declaration), repeat($.classifier_modifier)),
		interface_block: ($) => seq('{', repeat($.interface_member), '}'),

		// ---- class ----
		class_declaration: ($) => seq($.class_header, $.class_block),
		class_header: ($) =>
			seq(
				$.class_or_user,
				$.identifier,
				optional($.abstract_declaration),
				optional($.extends_declaration),
				optional($.implements_declaration),
				repeat($.class_service_modifier),
				repeat($.classifier_modifier),
			),
		class_or_user: ($) => choice('class', 'user'),
		class_service_modifier: ($) =>
			seq($.service_category_modifier, optional(seq('(', $.projection_reference, ')'))),
		service_category_modifier: ($) => choice('read', 'write', 'create', 'update', 'delete'),
		class_block: ($) => seq('{', repeat($.class_member), '}'),

		// ---- inheritance ----
		abstract_declaration: ($) => 'abstract',
		extends_declaration: ($) => seq('extends', $.class_reference),
		implements_declaration: ($) => seq('implements', sep1($.interface_reference, ',')),

		// ---- enumeration ----
		enumeration_declaration: ($) => seq('enumeration', $.identifier, $.enumeration_block),
		enumeration_block: ($) => seq('{', repeat($.enumeration_literal), '}'),
		enumeration_literal: ($) => seq($.identifier, optional(seq('(', $.enumeration_pretty_name, ')')), ','),
		enumeration_pretty_name: ($) => $.string_literal,

		// ---- association ----
		association_declaration: ($) => seq('association', $.identifier, $.association_block),
		association_block: ($) =>
			seq('{', optional($.association_end), optional($.association_end), optional($.relationship), '}'),
		association_end: ($) =>
			seq(
				$.identifier,
				':',
				$.class_reference,
				$.multiplicity,
				repeat($.association_end_modifier),
				optional($.order_by_declaration),
				';',
			),
		association_end_signature: ($) =>
			seq($.identifier, ':', $.classifier_reference, $.multiplicity, repeat($.association_end_modifier), ';'),
		relationship: ($) => seq('relationship', $.criteria_expression),

		// ---- projection ----
		projection_declaration: ($) =>
			seq(
				'projection',
				$.identifier,
				optional($.parameter_declaration_list),
				'on',
				$.classifier_reference,
				$.projection_block,
			),
		projection_block: ($) => seq('{', repeat($.projection_member), '}'),
		projection_member: ($) =>
			choice(
				$.projection_primitive_member,
				$.projection_reference_property,
				$.projection_parameterized_property,
				$.projection_projection_reference,
			),
		projection_primitive_member: ($) =>
			seq(optional(seq($.classifier_reference, '.')), $.identifier, ':', $.header, ','),
		projection_reference_property: ($) =>
			seq(optional(seq($.classifier_reference, '.')), $.identifier, ':', $.projection_block, ','),
		projection_projection_reference: ($) =>
			seq(optional(seq($.classifier_reference, '.')), $.identifier, ':', $.projection_reference, ','),
		projection_parameterized_property: ($) =>
			seq(
				optional(seq($.classifier_reference, '.')),
				$.identifier,
				$.argument_list,
				':',
				$.projection_block,
				',',
			),
		header: ($) => $.string_literal,

		// ---- service ----
		service_group_declaration: ($) => seq('service', $.identifier, 'on', $.class_reference, $.service_group_block),
		service_group_block: ($) => seq('{', repeat($.url_declaration), '}'),
		url_declaration: ($) => seq($.url, repeat1($.service_declaration)),
		url: ($) => seq(repeat1($.url_path_segment), optional('/'), optional($.query_parameter_list)),
		url_path_segment: ($) => seq('/', choice($.url_constant, $.url_parameter_declaration)),
		url_constant: ($) => choice($.identifier, $.url_identifier_token),
		query_parameter_list: ($) =>
			seq('?', $.url_parameter_declaration, repeat(seq('&', $.url_parameter_declaration))),
		url_parameter_declaration: ($) => seq('{', $.parameter_declaration, '}'),

		service_declaration: ($) => seq($.verb, $.service_block),
		// ANTLR's serviceBody is inlined here: every part is optional, and an empty
		// service block ("GET {}") is legal, so a standalone body rule would match
		// the empty string, which tree-sitter forbids.
		service_block: ($) =>
			seq(
				'{',
				optional($.service_multiplicity_declaration),
				repeat($.service_criteria_declaration),
				optional($.service_projection_dispatch),
				optional($.service_order_by_declaration),
				'}',
			),
		service_multiplicity_declaration: ($) => seq('multiplicity', ':', $.service_multiplicity, ';'),
		service_multiplicity: ($) => choice('one', 'many'),
		service_criteria_declaration: ($) => seq($.service_criteria_keyword, ':', $.criteria_expression, ';'),
		service_criteria_keyword: ($) => choice('criteria', 'authorize', 'validate', 'conflict'),
		service_projection_dispatch: ($) =>
			seq('projection', ':', $.projection_reference, optional($.argument_list), ';'),
		service_order_by_declaration: ($) => seq($.order_by_declaration, ';'),
		verb: ($) => choice('GET', 'POST', 'PUT', 'PATCH', 'DELETE'),

		// ---- member ----
		interface_member: ($) =>
			choice($.data_type_property, $.parameterized_property_signature, $.association_end_signature),
		class_member: ($) => choice($.data_type_property, $.parameterized_property, $.association_end_signature),
		data_type_property: ($) => choice($.primitive_property, $.enumeration_property),
		primitive_property: ($) =>
			seq(
				$.identifier,
				':',
				$.primitive_type,
				optional($.optional_marker),
				repeat($.data_type_property_modifier),
				repeat($.data_type_property_validation),
				';',
			),
		enumeration_property: ($) =>
			seq(
				$.identifier,
				':',
				$.enumeration_reference,
				optional($.optional_marker),
				repeat($.data_type_property_modifier),
				repeat($.data_type_property_validation),
				';',
			),
		parameterized_property: ($) =>
			seq(
				$.identifier,
				'(',
				optional(sep1($.parameter_declaration, ',')),
				')',
				':',
				$.class_reference,
				$.multiplicity,
				repeat($.parameterized_property_modifier),
				optional($.order_by_declaration),
				'{',
				$.criteria_expression,
				'}',
			),
		parameterized_property_signature: ($) =>
			seq(
				$.identifier,
				'(',
				optional(sep1($.parameter_declaration, ',')),
				')',
				':',
				$.classifier_reference,
				$.multiplicity,
				repeat($.parameterized_property_modifier),
				';',
			),
		optional_marker: ($) => '?',

		// ---- validations ----
		data_type_property_validation: ($) =>
			choice($.min_length_validation, $.max_length_validation, $.min_validation, $.max_validation),
		min_length_validation: ($) => seq($.min_length_validation_keyword, $.integer_validation_parameter),
		max_length_validation: ($) => seq($.max_length_validation_keyword, $.integer_validation_parameter),
		min_validation: ($) => seq($.min_validation_keyword, $.integer_validation_parameter),
		max_validation: ($) => seq($.max_validation_keyword, $.integer_validation_parameter),
		integer_validation_parameter: ($) => seq('(', $.integer_literal, ')'),
		min_length_validation_keyword: ($) => choice('minLength', 'minimumLength'),
		max_length_validation_keyword: ($) => choice('maxLength', 'maximumLength'),
		min_validation_keyword: ($) => choice('min', 'minimum'),
		max_validation_keyword: ($) => choice('max', 'maximum'),

		// ---- parameter ----
		parameter_declaration: ($) => choice($.primitive_parameter_declaration, $.enumeration_parameter_declaration),
		primitive_parameter_declaration: ($) =>
			seq($.identifier, ':', $.primitive_type, $.multiplicity, repeat($.parameter_modifier)),
		enumeration_parameter_declaration: ($) =>
			seq($.identifier, ':', $.enumeration_reference, $.multiplicity, repeat($.parameter_modifier)),
		parameter_declaration_list: ($) => seq('(', sep1($.parameter_declaration, ','), ')'),

		// ---- argument ----
		argument_list: ($) => seq('(', optional(sep1($.argument, ',')), ')'),
		argument: ($) => choice($.literal, $.literal_list, $.native_literal, $.parameter_reference),

		// ---- multiplicity ----
		multiplicity: ($) => seq('[', $.multiplicity_body, ']'),
		multiplicity_body: ($) =>
			seq(
				field('lower_bound', $.integer_literal_token),
				'..',
				field('upper_bound', choice($.integer_literal_token, '*')),
			),

		// Prefer reading a known primitive-type keyword as a primitive_type rather
		// than an identifier when both are possible (e.g. the type in "x : Boolean;").
		primitive_type: ($) =>
			prec(
				1,
				choice(
					'Boolean',
					'Integer',
					'Long',
					'Double',
					'Float',
					'String',
					'Instant',
					'LocalDate',
					'TemporalInstant',
					'TemporalRange',
				),
			),

		// ---- modifiers ----
		classifier_modifier: ($) =>
			choice('systemTemporal', 'validTemporal', 'bitemporal', 'versioned', 'audited', 'transient'),
		data_type_property_modifier: ($) =>
			choice(
				'key',
				'private',
				'userId',
				'id',
				'valid',
				'system',
				'from',
				'to',
				'createdBy',
				'createdOn',
				'lastUpdatedBy',
				'version',
				'derived',
				'final',
			),
		association_end_modifier: ($) => choice('owned', 'final', 'version', 'private', 'createdBy', 'lastUpdatedBy'),
		parameterized_property_modifier: ($) => choice('createdBy', 'lastUpdatedBy'),
		parameter_modifier: ($) => choice('version', 'userId', 'id'),

		// ---- order by ----
		order_by_declaration: ($) => seq('orderBy', ':', sep1($.order_by_member_reference_path, ',')),
		order_by_member_reference_path: ($) => seq($.this_member_reference_path, optional($.order_by_direction)),
		order_by_direction: ($) => choice('ascending', 'descending'),

		// ---- criteria ----
		// ANTLR criteriaExpression is left recursive with && (higher) then ||.
		criteria_expression: ($) =>
			choice(
				$.criteria_expression_and,
				$.criteria_expression_or,
				$.criteria_expression_group,
				$.criteria_all,
				$.criteria_operator,
				$.criteria_edge_point,
				$.criteria_native,
			),
		criteria_expression_and: ($) =>
			prec.left(2, seq(field('left', $.criteria_expression), '&&', field('right', $.criteria_expression))),
		criteria_expression_or: ($) =>
			prec.left(1, seq(field('left', $.criteria_expression), '||', field('right', $.criteria_expression))),
		criteria_expression_group: ($) => seq('(', $.criteria_expression, ')'),
		criteria_all: ($) => 'all',
		criteria_operator: ($) =>
			seq(field('source', $.expression_value), $.operator, field('target', $.expression_value)),
		criteria_edge_point: ($) => seq($.expression_member_reference, 'equalsEdgePoint'),
		criteria_native: ($) => seq('native', '(', $.identifier, ')'),

		expression_value: ($) =>
			choice(
				$.literal,
				$.literal_list,
				$.this_member_reference_path,
				$.type_member_reference_path,
				$.native_literal,
				$.parameter_reference,
			),
		expression_member_reference: ($) => choice($.this_member_reference_path, $.type_member_reference_path),
		literal_list: ($) => seq('(', sep1($.literal, ','), ')'),
		// 'user' in an expression-value position is the native literal, not an
		// identifier; prefer that reading (matches ANTLR's dedicated alternative).
		native_literal: ($) => prec(1, 'user'),

		operator: ($) => choice($.equality_operator, $.inequality_operator, $.in_operator, $.string_operator),
		equality_operator: ($) => choice('==', '!='),
		inequality_operator: ($) => choice('<', '>', '<=', '>='),
		in_operator: ($) => 'in',
		string_operator: ($) => choice('contains', 'startsWith', 'endsWith'),

		// ---- references ----
		interface_reference: ($) => $.identifier,
		class_reference: ($) => $.identifier,
		classifier_reference: ($) => $.identifier,
		enumeration_reference: ($) => $.identifier,
		projection_reference: ($) => $.identifier,
		member_reference: ($) => $.identifier,
		association_end_reference: ($) => $.identifier,
		parameter_reference: ($) => $.identifier,

		this_member_reference_path: ($) =>
			seq('this', repeat(seq('.', $.association_end_reference)), '.', $.member_reference),
		type_member_reference_path: ($) =>
			seq($.class_reference, repeat(seq('.', $.association_end_reference)), '.', $.member_reference),

		// ---- identifier ----
		identifier: ($) => choice($.identifier_token, $.keyword_valid_as_identifier),
		keyword_valid_as_identifier: ($) =>
			choice(
				'package',
				'enumeration',
				'interface',
				'class',
				'association',
				'projection',
				'service',
				'user',
				'abstract',
				'extends',
				'implements',
				'native',
				'relationship',
				'multiplicity',
				'orderBy',
				'criteria',
				'authorize',
				'validate',
				'conflict',
				// classifierModifier
				'systemTemporal',
				'validTemporal',
				'bitemporal',
				'versioned',
				'audited',
				'transient',
				// dataTypePropertyModifier
				'key',
				'private',
				'userId',
				'id',
				'valid',
				'system',
				'from',
				'to',
				'createdBy',
				'createdOn',
				'lastUpdatedBy',
				'version',
				'derived',
				// associationEndModifier
				'owned',
				'final',
				// verbs
				'GET',
				'POST',
				'PUT',
				'PATCH',
				'DELETE',
				// serviceCategoryModifier
				'read',
				'write',
				'create',
				'update',
				'delete',
				// inOperator / stringOperator
				'in',
				'contains',
				'startsWith',
				'endsWith',
				// primitiveType
				'Boolean',
				'Integer',
				'Long',
				'Double',
				'Float',
				'String',
				'Instant',
				'LocalDate',
				'TemporalInstant',
				'TemporalRange',
			),

		// ---- literals ----
		literal: ($) =>
			choice(
				$.integer_literal,
				$.floating_point_literal,
				$.boolean_literal,
				$.character_literal,
				$.string_literal,
				$.null_literal,
			),
		integer_literal: ($) => $.integer_literal_token,
		floating_point_literal: ($) => $.floating_point_literal_token,
		boolean_literal: ($) => choice('true', 'false'),
		character_literal: ($) => $.character_literal_token,
		string_literal: ($) => $.string_literal_token,
		null_literal: ($) => 'null',

		// ---- lexer tokens ----
		// Identifier: JavaLetter JavaLetterOrDigit* (ASCII letters/$/_ plus non-ASCII code points).
		identifier_token: ($) => /[a-zA-Z_$\u0080-\uFFFF][a-zA-Z0-9_$\u0080-\uFFFF]*/,

		// UrlIdentifier: [a-zA-Z][a-zA-Z0-9_]*'-'[a-zA-Z0-9_-]* (must contain a dash).
		url_identifier_token: ($) => /[a-zA-Z][a-zA-Z0-9_]*-[a-zA-Z0-9_-]*/,

		// Integer/floating tokens (decimal, hex, octal, binary; underscores allowed).
		integer_literal_token: ($) =>
			token(choice(/0[xX][0-9a-fA-F_]+[lL]?/, /0[bB][01_]+[lL]?/, /0[0-7_]+[lL]?/, /(0|[1-9][0-9_]*)[lL]?/)),
		floating_point_literal_token: ($) =>
			token(
				choice(
					/[0-9][0-9_]*\.[0-9_]*([eE][+-]?[0-9_]+)?[fFdD]?/,
					/\.[0-9][0-9_]*([eE][+-]?[0-9_]+)?[fFdD]?/,
					/[0-9][0-9_]*[eE][+-]?[0-9_]+[fFdD]?/,
					/[0-9][0-9_]*[fFdD]/,
				),
			),

		character_literal_token: ($) =>
			token(seq("'", choice(/[^'\\\r\n]/, /\\['btnfr"\\]/, /\\[0-3]?[0-7]?[0-7]/, /\\u+[0-9a-fA-F]{4}/), "'")),
		string_literal_token: ($) =>
			token(
				seq(
					'"',
					repeat(choice(/[^"\\\r\n]/, /\\['btnfr"\\]/, /\\[0-3]?[0-7]?[0-7]/, /\\u+[0-9a-fA-F]{4}/)),
					'"',
				),
			),

		// ---- comments (extras) ----
		line_comment: ($) => token(seq('//', /[^\r\n]*/)),
		block_comment: ($) => token(seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')),
	},
});

// identifier ('.'-or-','-etc-separated) list, one or more.
function sep1(rule, separator) {
	return seq(rule, repeat(seq(separator, rule)));
}
