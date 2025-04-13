use crate::{MaybeError, OkMaybe};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    str::FromStr,
};

/// Guarantees all attributes (`<k> = <v>` or `<v>`) are specified only once
///
/// Raises an error for each duplicated attribute `#[cache_diff(ignore, ignore)]`.
/// If no duplicate attributes returns a lookup by enum discriminant with span information.
///
/// ```rust
#[doc = include_str!("./fixtures/hidden_parse_attribute_rename_ignore.md")]
///
/// let mut errors = proc_micro::MaybeError::new();
///
/// let _unique = proc_micro::unique::<ParseAttribute>(
///     proc_micro::parse_attrs(
///         &proc_micro::AttrNamespace("macro_name"),
///         &syn_field(syn::parse_quote! {
///              #[macro_name(rename = "Ruby version", rename = "Specified rename twice, oops")]
///              version: String
///         }).attrs
///     ).push_unwrap(&mut errors)
/// ).push_unwrap(&mut errors);
///
/// let error = errors.maybe().unwrap();
/// assert_eq!(
///     vec![
///         "Duplicate attribute: `rename`".to_string(),
///         "previously `rename` defined here".to_string()
///     ],
///     error
///         .into_iter()
///         .map(|e| format!("{e}"))
///         .collect::<Vec<_>>()
/// );
/// ```
#[cfg(feature = "strum")]
pub fn unique<T>(
    parsed_attributes: impl IntoIterator<Item = WithSpan<T>>,
) -> OkMaybe<HashMap<T::Discriminant, WithSpan<T>>, syn::Error>
where
    T: strum::IntoDiscriminant,
    T::Discriminant: Eq + Display + std::hash::Hash + Copy,
{
    let mut seen = HashMap::new();
    let mut errors = MaybeError::new();
    for attribute in parsed_attributes {
        let WithSpan(ref parsed, span) = attribute;
        let key = parsed.discriminant();
        if let Some(WithSpan(_, prior)) = seen.insert(key, attribute) {
            errors.push_back(syn::Error::new(
                span,
                format!("Duplicate attribute: `{key}`"),
            ));
            errors.push_back(syn::Error::new(
                prior,
                format!("previously `{key}` defined here"),
            ));
        }
    }

    OkMaybe(seen, errors.maybe())
}

/// Check exclusive attributes
///
/// Errors if an exclusive attribute is used with any other attributes.
/// For example `ignore` would negate any other attributes so it is
/// mutually exclusive.
///
/// Does NOT check for repeated attributes for that, use [`unique`]
///
/// ```rust
#[doc = include_str!("./fixtures/hidden_parse_attribute_rename_ignore.md")]
///
/// let mut errors = proc_micro::MaybeError::new();
///
/// let attributes = proc_micro::parse_attrs(
///     &proc_micro::AttrNamespace("macro_name"),
///     &syn_field(syn::parse_quote! {
///          #[macro_name(ignore, rename = "Ruby version")]
///          version: String
///     }).attrs
/// ).push_unwrap(&mut errors);
///
/// proc_micro::check_exclusive::<ParseAttribute>(
///     KnownAttribute::ignore,
///     &attributes
/// ).push_unwrap(&mut errors);
///
/// let error = errors.maybe().unwrap();
/// assert_eq!(
///     vec![
///         "Exclusive attribute. Remove either `ignore` or `rename`".to_string(),
///         "cannot be used with `ignore`".to_string()
///     ],
///     error
///         .into_iter()
///         .map(|e| format!("{e}"))
///         .collect::<Vec<_>>()
/// );
/// ```
#[cfg(feature = "strum")]
pub fn check_exclusive<T>(
    exclusive: T::Discriminant,
    collection: &[WithSpan<T>],
) -> OkMaybe<(), syn::Error>
where
    T: strum::IntoDiscriminant + syn::parse::Parse,
    T::Discriminant: Eq + Display + std::hash::Hash + Copy,
{
    let mut errors = MaybeError::new();
    let mut keys = collection
        .iter()
        .map(|WithSpan(value, _)| value.discriminant())
        .collect::<HashSet<T::Discriminant>>();

    if keys.remove(&exclusive) && !keys.is_empty() {
        let other_keys = keys
            .iter()
            .map(|key| format!("`{key}`"))
            .collect::<Vec<_>>()
            .join(", ");

        for WithSpan(value, span) in collection {
            if value.discriminant() == exclusive {
                errors.push_front(syn::Error::new(
                    *span,
                    format!("Exclusive attribute. Remove either `{exclusive}` or {other_keys}",),
                ))
            } else {
                errors.push_back(syn::Error::new(
                    *span,
                    format!("cannot be used with `{exclusive}`"),
                ))
            }
        }
    }

    OkMaybe((), errors.maybe())
}

/// Parses one bare word like "rename" for any iterable enum, and that's it
///
/// Won't parse an equal sign or anything else. Emits all known keys for
/// debugging help when an unknown string is passed in.
///
/// Can be used to derive [syn::parse::Parse] for the discriminant or the
/// enum directly.
///
/// ```
/// assert_eq!(KnownAttribute::ignore, syn::parse_str("ignore").unwrap());
/// assert_eq!(KnownAttribute::rename, syn::parse_str("rename").unwrap());
/// assert!(syn::parse_str::<KnownAttribute>("unknown").is_err());
///
/// impl syn::parse::Parse for KnownAttribute {
///     fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
///         let ident = input.parse::<syn::Ident>()?;
///         proc_micro::known_attribute(&ident)
///     }
/// }
///
/// #[derive(strum::EnumDiscriminants, Debug, PartialEq)]
/// #[strum_discriminants(
///     name(KnownAttribute),
///     derive(strum::EnumIter, strum::Display, strum::EnumString, Hash)
/// )]
/// enum ParseAttribute {
///     #[allow(non_camel_case_types)]
///     # #[allow(dead_code)]
///     rename(String),
///     #[allow(non_camel_case_types)]
///     # #[allow(dead_code)]
///     ignore,
/// }
/// ```
#[cfg(feature = "strum")]
pub fn known_attribute<T>(identity: &syn::Ident) -> Result<T, syn::Error>
where
    T: FromStr + strum::IntoEnumIterator + Display,
{
    let name_str = &identity.to_string();
    T::from_str(name_str).map_err(|_| {
        syn::Error::new(
            identity.span(),
            format!(
                "Unknown attribute: `{identity}`. Must be one of {valid_keys}",
                valid_keys = T::iter()
                    .map(|key| format!("`{key}`"))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        )
    })
}

/// Parse attributes into a vector
///
/// Returns at least one error per attribute block `#[attribute(...)]` if it cannot
/// be parsed.
/// Check exclusive attributes
///
/// Errors if an exclusive attribute is used with any other attributes.
/// For example `ignore` would negate any other attributes so it is
/// mutually exclusive.
///
/// Does NOT check for repeated attributes for that, use [`unique`]
///
/// ```rust
#[doc = include_str!("./fixtures/hidden_parse_attribute_rename_ignore.md")]
///
/// let mut errors = proc_micro::MaybeError::new();
///
/// let attributes = proc_micro::parse_attrs::<ParseAttribute>(
///     &proc_micro::AttrNamespace("macro_name"),
///     &syn_field(syn::parse_quote! {
///          #[macro_name(ignore, rename = "Ruby version")]
///          version: String
///     }).attrs
/// ).push_unwrap(&mut errors);
///
/// assert!(errors.maybe().is_none());
/// assert_eq!(
///     vec![
///         ParseAttribute::ignore,
///         ParseAttribute::rename("Ruby version".to_string())
///     ],
///     attributes
/// );
/// ```
///
/// Commonly used along with [WithSpan] to return the span of the parsed attribute:
///
/// ```rust
/// use proc_micro::WithSpan;
#[doc = include_str!("./fixtures/hidden_parse_attribute_rename_ignore.md")]
///
/// let mut errors = proc_micro::MaybeError::new();
///
/// let attributes = proc_micro::parse_attrs::<WithSpan<ParseAttribute>>(
///     &proc_micro::AttrNamespace("macro_name"),
///     &syn_field(syn::parse_quote! {
///          #[macro_name(ignore, rename = "Ruby version")]
///          version: String
///     }).attrs
/// ).push_unwrap(&mut errors);
///
/// assert!(matches!(attributes.first(), Some(WithSpan(ParseAttribute::ignore, _))));
/// ```
pub fn parse_attrs<T>(
    namespace: &AttrNamespace,
    attrs: &[syn::Attribute],
) -> OkMaybe<Vec<T>, syn::Error>
where
    T: syn::parse::Parse,
{
    let mut attributes = Vec::new();
    let mut errors = MaybeError::new();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident(namespace)) {
        match attr
            .parse_args_with(syn::punctuated::Punctuated::<T, syn::Token![,]>::parse_terminated)
        {
            Ok(attrs) => {
                for attribute in attrs {
                    attributes.push(attribute);
                }
            }
            Err(error) => errors.push_back(error),
        }
    }

    OkMaybe(attributes, errors.maybe())
}

/// Helper type for parsing a type and preserving the original span
///
/// Used with [syn::punctuated::Punctuated] to capture the inner span of an attribute.
#[derive(Debug)]
pub struct WithSpan<T>(pub T, pub proc_macro2::Span);

impl<T: syn::parse::Parse> syn::parse::Parse for WithSpan<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        Ok(WithSpan(input.parse()?, span))
    }
}

/// Represents the namespace of macro attributes
///
/// For example, a macro with a namespace of "my_macro" would look like this:
///
/// ```text
/// #[my_macro(...)]
/// ```
///
/// This can be represented via:
///
/// ```rust
/// # #[allow(dead_code)]
/// const NAMESPACE: proc_micro::AttrNamespace =
///     proc_micro::AttrNamespace("my_macro");
/// ```
#[derive(Debug, Clone)]
pub struct AttrNamespace<'a>(pub &'a str);

impl AttrNamespace<'_> {
    #[cfg(feature = "strum")]
    pub fn parse_attrs<T>(&self, attrs: &[syn::Attribute]) -> OkMaybe<Vec<T>, syn::Error>
    where
        T: syn::parse::Parse,
    {
        crate::parse_attrs(self, attrs)
    }
}

impl AsRef<str> for AttrNamespace<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> std::ops::Deref for AttrNamespace<'a> {
    type Target = &'a str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for AttrNamespace<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
