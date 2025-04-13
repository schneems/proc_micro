<!-- cargo-rdme start -->

# Proc Micro

Small conveniences for high-quality macros.

## Use

```term
$ cargo add proc_micro
$ cargo add strum --features=derive
```

## Errors

Normal rust code returns on the first error. Great macros accumulate as many
errors as they can and show them all at once.

- [MaybeError]: A container that holds zero or more [syn::Error]-s. When
  it holds error(s) they can be accumulated into a single [syn::Error] (which
  is a data structure for holding one or more macro errors).
- [OkMaybe]: An alternative for Result that allows for returning both data
  and (maybe) errors at the same time. Use Result when an error means
  the data is unusable (untrustable) or [OkMaybe] when an error means
  the partial data might still be useful in generating additional error.
  information. The caller can convert an [OkMaybe] into a [Result], but cannot
  convert a [Result] into an [OkMaybe].

## Enum powered Attribute parsing

These helpers work with attributes defined as enums with this library:

- [WithSpan]: Holds the attributes you parsed and their spans
- [parse_attrs]: Turns a [syn::Attribute] into a [Vec] of your parsed enums
- [unique]: Guarantee each enum attribute is only listed once
- [check_exclusive]: Check if an exclusive attribute is used in conjunction with others.
- [known_attribute]: Convienence function for parsing an enum discriminant

## Tutorial

Here's how you define a macro attribute that has a namespace of `my_macro` and
accepts `rename = <string>` and `ignore` attributes using the [strum crate](https://crates.io/crates/strum):

```rust
const NAMESPACE: proc_micro::AttrNamespace =
    proc_micro::AttrNamespace("my_macro");

#[derive(strum::EnumDiscriminants, Debug, PartialEq)]
#[strum_discriminants(
    name(KnownAttribute),
    derive(strum::EnumIter, strum::Display, strum::EnumString, Hash)
)]
enum ParseAttribute {
    // #[my_macro(rename = "<string>")]
    #[allow(non_camel_case_types)]
    rename(String),
    // #[my_macro(ignore)]
    #[allow(non_camel_case_types)]
    ignore,
}

impl syn::parse::Parse for ParseAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        match proc_micro::known_attribute(&ident)? {
            KnownAttribute::rename => {
                input.parse::<syn::Token![=]>()?;
                Ok(ParseAttribute::rename(
                    input.parse::<syn::LitStr>()?.value(),
                ))
            }
            KnownAttribute::ignore => Ok(ParseAttribute::ignore),
        }
    }
}
```

Each parsed attribute is stored in our enum while the discriminant can be
used as a lookup. By representing attributes as an enum, we can be confident
our code handles attribute additions or modifications exhaustively.

This also provides a platform for unit testing attribute logic:

```rust
let attribute = syn::parse_str::<ParseAttribute>(
    "rename = \"Ruby version\""
).unwrap();
assert_eq!(ParseAttribute::rename("Ruby version".to_string()), attribute);
```

Then within your macro code you can convert many comma separated attributes
into enums while accumulating errors:

```rust

let mut errors = proc_micro::MaybeError::new();

let field: syn::Field = syn::parse_quote! {
    #[my_macro(ignore, rename = "Ruby version")]
    version: String
};

let attributes: Vec<WithSpan<ParseAttribute>> = proc_micro::parse_attrs(
    &NAMESPACE, &field.attrs
).push_unwrap(&mut errors);

assert_eq!(2, attributes.len());
assert!(matches!(attributes.first(), Some(WithSpan(ParseAttribute::ignore, _))));
assert!(errors.is_empty());
```

Use this result with other helpers to validate your attribute requirements.
For example [unique] requires that attributes are specified at most once i.e.
`#[my_macro(ignore, ignore)]` is incorrect. And [check_exclusive] is called
for attributes that must be used exclusively, i.e. using "ignore" with any
other attribute is in valid as they would have no effect. And you can use
the returned [WithSpan] information to build your own custom [syn::Error]
errors.

```rust

use proc_micro::OkMaybe;

// Make a structure to store your parsed configuration
#[derive(Debug, Clone)]
struct FieldConfig {
    ignore: bool,
    rename: Option<String>
}

// Use our building blocks to implement your desired logic
fn field_config(field: &syn::Field) -> OkMaybe<FieldConfig, syn::Error> {
    let mut rename_config = None;
    let mut ignore_config = false;
    let mut errors = proc_micro::MaybeError::new();

    let attributes: Vec<WithSpan<ParseAttribute>> = proc_micro::parse_attrs(
        &NAMESPACE, &field.attrs
    ).push_unwrap(&mut errors);

    proc_micro::check_exclusive(KnownAttribute::ignore, &attributes)
        .push_unwrap(&mut errors);
    let mut unique = proc_micro::unique(attributes)
        .push_unwrap(&mut errors);

    for (_, WithSpan(attribute, _)) in unique.drain() {
        match attribute {
            ParseAttribute::ignore => ignore_config = true,
            ParseAttribute::rename(name) => rename_config = Some(name),
        }
    }

    OkMaybe(
        FieldConfig {
            ignore: ignore_config,
            rename: rename_config
        },
        errors.maybe()
    )
}

// No problems
let _config = field_config(&syn::parse_quote! {
    #[my_macro(rename = "Ruby version")]
    version: String
}).to_result().unwrap();

// Problem with `check_exclusive`
let result = field_config(&syn::parse_quote! {
    #[my_macro(rename = "Ruby version", ignore)]
    version: String
}).to_result();

 assert!(result.is_err(), "Expected to be err but is {result:?}");
 let err = result.err().unwrap();
 assert_eq!(vec![
    "Exclusive attribute. Remove either `ignore` or `rename`".to_string(),
    "cannot be used with `ignore`".to_string()],
    err.into_iter().map(|e| e.to_string()).collect::<Vec<String>>()
 );

// Problem with `unique`
let result = field_config(&syn::parse_quote! {
    #[my_macro(ignore, ignore)]
    version: String
}).to_result();

assert!(result.is_err(), "Expected to be err but is {result:?}");
let err = result.err().unwrap();
assert_eq!(vec![
    "Duplicate attribute: `ignore`".to_string(),
    "previously `ignore` defined here".to_string()],
    err.into_iter().map(|e| e.to_string()).collect::<Vec<String>>()
);

// Multiple problems `unique` and unknown attribute
let result = field_config(&syn::parse_quote! {
    #[my_macro(ignore, ignore)]
    #[my_macro(unknown)]
    version: String
}).to_result();

assert!(result.is_err(), "Expected to be err but is {result:?}");
let err = result.err().unwrap();
assert_eq!(vec![
    "Unknown attribute: `unknown`. Must be one of `rename`, `ignore`".to_string(),
    "Duplicate attribute: `ignore`".to_string(),
    "previously `ignore` defined here".to_string(),
   ],
    err.into_iter().map(|e| e.to_string()).collect::<Vec<String>>()
);
```

<!-- cargo-rdme end -->
