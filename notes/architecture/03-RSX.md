# RSX and Autofmt Architecture

The `dioxus-rsx` crate parses JSX-like syntax into Rust code, while `dioxus-autofmt` provides formatting capabilities.

## RSX Parsing

### Entry Point: CallBody
Root struct for `rsx! {}` macro contents:
```
CallBody
├── TemplateBody (list of BodyNode roots)
├── template_idx: Cell<usize>
└── span: Option<Span>
```

`CallBody::new()` initializes template indices and cascades hotreload info through nested structures.

### BodyNode Enum
Six variants representing RSX content:
- `Element(Element)` - HTML elements (div, span)
- `Component(Component)` - User components
- `Text(TextNode)` - String literals with interpolation
- `RawExpr(ExprNode)` - Braced expressions `{expr}`
- `ForLoop(ForLoop)` - `for pat in expr { body }`
- `IfChain(IfChain)` - `if cond { } else { }`

### Parsing Priority
1. Peek for LitStr → TextNode
2. Peek for `for` → ForLoop
3. Peek for `if` → IfChain
4. Peek for `match` → RawExpr
5. Peek for Brace → RawExpr
6. Web components: Ident + `-` → Element
7. Lowercase ident → Element
8. Otherwise → Component (fallback)

## Key AST Types

### Element
```
Element
├── name: ElementName (Ident or Custom)
├── raw_attributes: Vec<Attribute>
├── merged_attributes: Vec<Attribute>  // After combining duplicates
├── spreads: Vec<Spread>
├── children: Vec<BodyNode>
├── brace: Option<Brace>
└── diagnostics: Diagnostics
```

`merge_attributes()` collapses duplicate attribute names using `IfmtInput` with space delimiter.

### Attribute
```
Attribute
├── name: AttributeName (BuiltIn, Custom, or Spread)
├── value: AttributeValue
├── colon: Option<Token![:]>
├── dyn_idx: DynIdx
└── el_name: Option<ElementName>
```

### AttributeValue Variants
- `Shorthand(Ident)` - attribute without value
- `AttrLiteral(HotLiteral)` - hotreloadable literal
- `EventTokens(PartialClosure)` - event handlers
- `IfExpr(IfAttributeValue)` - conditional attributes
- `AttrExpr(PartialExpr)` - arbitrary expressions

### HotLiteral
```
HotLiteral
├── Fmted(HotReloadFormattedSegment)  // "{expr}" interpolation
├── Float(LitFloat)
├── Int(LitInt)
└── Bool(LitBool)
```

### IfmtInput (Formatted Strings)
Parses string contents into segments:
- `Segment::Literal(String)` - plain text
- `Segment::Formatted(FormattedSegment)` - `{expr}` interpolation

Parsing rules:
- `{{` → literal `{`
- `}}` → literal `}`
- `{expr}` → formatted segment
- `{expr:format_args}` → formatted with format spec

### Component
```
Component
├── name: syn::Path
├── generics: Option<AngleBracketedGenericArguments>
├── fields: Vec<Attribute>
├── component_literal_dyn_idx: Vec<DynIdx>
├── spreads: Vec<Spread>
├── children: TemplateBody
├── dyn_idx: DynIdx
└── diagnostics: Diagnostics
```

### ForLoop
```
ForLoop
├── for_token, pat, in_token
├── expr: Box<Expr>
├── body: TemplateBody
└── dyn_idx: DynIdx
```
Generates: `(expr).into_iter().map(|pat| { body })`

### IfChain
```
IfChain
├── if_token, cond: Box<Expr>
├── then_branch: TemplateBody
├── else_if_branch: Option<Box<IfChain>>
├── else_branch: Option<TemplateBody>
└── dyn_idx: DynIdx
```

### DynIdx
`Cell<Option<usize>>` for tracking dynamic indices. Transparent in PartialEq/Eq/Hash. Used for hot-reload mapping.

## Code Generation

### TemplateBody → VNode::Template
Generated output structure:
1. `__TEMPLATE_ROOTS` - Static array of TemplateNode
2. **Dynamic nodes** - Components, interpolated text, loops, conditionals
3. **Dynamic attributes** - Non-static attribute values
4. **Dynamic literal pool** - In debug, vec of formatted values
5. **Dynamic value pool** - Maps literal indices to values

### Template Structure
```rust
dioxus_core::Element::Ok({
    #[cfg(debug_assertions)]
    fn __original_template() -> &'static HotReloadedTemplate { ... }

    let __dynamic_nodes: [DynamicNode; N] = [ ... ];
    let __dynamic_attributes: [Box<[Attribute]>; M] = [ ... ];
    static __TEMPLATE_ROOTS: &[TemplateNode] = &[ ... ];

    // Template reference and rendering
})
```

### Generated TemplateNode Types
- `Element { tag, namespace, attrs, children }` - Static element
- `Text { text }` - Static text
- `Dynamic { id }` - References dynamic node pool

## Template System

### TemplateBody Structure
```
TemplateBody
├── roots: Vec<BodyNode>
├── template_idx: DynIdx
├── node_paths: Vec<Vec<u8>>      // Path to each dynamic node
├── attr_paths: Vec<(Vec<u8>, usize)>  // Path and attribute index
├── dynamic_text_segments: Vec<FormattedSegment>
└── diagnostics: Diagnostics
```

### Template ID Assignment
- `CallBody::next_template_idx()` generates sequential IDs
- Each nested structure gets unique ID
- Combined with `file!()`, `line!()`, `column!()` for source location

### HotReloadFormattedSegment
Wraps `IfmtInput` with `dynamic_node_indexes: Vec<DynIdx>`:
- One DynIdx per `Segment::Formatted` entry
- Maps formatted segments to dynamic nodes during hot-reload

## Autofmt System

### Entry Points
- `try_fmt_file(contents, &syn::File, IndentOptions)` → Vec<FormattedBlock>
- `fmt_block(block_str, indent_level, IndentOptions)` → Option<String>
- `write_block_out(body)` → Option<String>

### FormattedBlock
```
FormattedBlock
├── formatted: String
├── start: usize (byte offset)
└── end: usize (byte offset)
```

### Writer State
```
Writer
├── raw_src: &str
├── src: Vec<&str>  // Lines
├── out: Buffer
├── cached_formats: HashMap<LineColumn, String>
└── invalid_exprs: Vec<Span>
```

### Buffer
```
Buffer
├── buf: String
├── indent_level: usize
└── indent: IndentOptions
```

### Optimization Levels
1. **Empty**: `div {}` (no space inside)
2. **Oneliner**: `div { class: "x", child {} }` (single line)
3. **PropsOnTop**: Props multiline, children follow
4. **NoOpt**: Everything multiline

### Short-Circuit Optimization
```rust
if formatted.len() <= 80
    && !formatted.contains('\n')
    && !body_is_solo_expr
    && !formatted.trim().is_empty()
{
    formatted = format!(" {formatted} ");  // Collapse to single line
}
```

## Whitespace Handling

Whitespace is significant in RSX:
- Text nodes preserve exact whitespace
- Comments must be preserved
- Formatting must not change text node content

### Comment Preservation
- `write_comments()` accumulates full-line comments before spans
- `write_inline_comments()` preserves end-of-line comments
- Comments tracked via `LineColumn` from Span

## Expression Formatting

### write_partial_expr() Strategy
1. Use `prettier_please` to unparse expressions
2. Handle nested rsx! macros specially
3. `unparse_expr()` visits macro calls:
   - Format nested rsx! blocks
   - Replace macros with marker unicode
   - Apply formatted blocks back

### Marker Replacement
Uses marker `"𝕣𝕤𝕩"` to replace macros during unparse, avoiding conflicts with actual code.

## Indentation System

### IndentOptions
```
IndentOptions
├── width: usize
├── indent_string: String ("\t" or spaces)
└── split_line_attributes: bool
```

### Functions
- `indent_str()` → returns indent string
- `count_indents(line)` → estimates indent level
- `line_length(line)` → estimates visible length

## Attribute Formatting

### write_attributes(props_same_line)
- `true`: attrs on one line with spaces
- `false`: each attr on new line, indented

### write_attribute_value()
- `Shorthand` → just ident
- `AttrLiteral` → uses Display impl
- `EventTokens` → write_partial_expr()
- `IfExpr` → write_attribute_if_chain()
- `AttrExpr` → write_partial_expr()

## Extension Points

### Adding New Node Types
1. Add variant to BodyNode enum
2. Implement Parse trait
3. Implement ToTokens for code gen
4. Add write_* method to Writer

### Adding New Attribute Types
1. Add variant to AttributeValue
2. Implement Parse for detection
3. Add merge handling if needed
4. Add write_attribute_value case

### Changing Formatting Heuristics
- Modify ShortOptimization logic
- Adjust threshold constants (80, 100 chars)
- Modify estimation functions
