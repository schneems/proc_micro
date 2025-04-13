use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};

/// Represents generic data and maybe an error
///
/// - Full data when err is empty `OkMaybe(_, None)`.
/// - Maybe partial data when err is present: `OkMaybe(_, Some(_))`.
///
/// ## Why
///
/// When parsing, we want to accumulate as many errors as possible. That means
/// that even partial data may be useful. For example, this code has the same
/// attribute specified twice and also a syntax error that will prevent it from
/// parsing additional attributes:
///
/// ```text
/// #[macro_name(unknown attribute here, ignore)]
/// #[macro_name(rename = "First", rename = "Again")]
/// ```
///
/// In this situation `unknown attribute here` constitutes a syntax error that would
/// prevent parsing `ignore`. However, the second line can be fully parsed but
/// we want to error on the accidental duplicate `rename` attribute. By returning
/// partial data along side of errors we can report
///
/// ## Examples
///
/// If `E` implements [IntoIterator] and [Extend] (as [syn::Error] or does) then you can
/// accumulate errors using [OkMaybe::push_unwrap]. Use this when partial data
/// returned might hold additional errors that you wish to accumulate before returning.
///
/// For example:
///
/// ```rust
/// use proc_micro::{OkMaybe, MaybeError};
///
/// let mut errors = MaybeError::new();
///
/// # let span = proc_macro2::Span::call_site();
/// let _out: () =
///     OkMaybe((), Some(syn::Error::new(span, "message".to_string())))
///         .push_unwrap(&mut errors);
///
/// assert!(errors.has_err());
/// assert!(matches!(errors.maybe(), Some(_)));
/// ```
///
/// For situations where partial data is not usable, convert into a result:
///
/// ```rust
/// use proc_micro::OkMaybe;
///
/// # let span = proc_macro2::Span::call_site();
/// let result: Result<(), syn::Error> =
///     OkMaybe((), Some(syn::Error::new(span, "message".to_string())))
///         .to_result();
///
/// assert!(result.is_err(), "Expected an error, got {:?}", result);
///
/// let result: Result<(), syn::Error> =
///     OkMaybe((), None)
///         .to_result();
/// assert!(result.is_ok(), "Expected Ok, got {:?}", result);
/// ```
#[derive(Debug)]
pub struct OkMaybe<T, E>(pub T, pub Option<E>);
impl<T, E> OkMaybe<T, E> {
    pub fn to_result(self) -> Result<T, E> {
        let OkMaybe(ok, maybe) = self;
        if let Some(e) = maybe { Err(e) } else { Ok(ok) }
    }
}

impl<T, E> OkMaybe<T, E>
where
    E: IntoIterator<Item = E>,
{
    /// If E implements [IntoIterator] and [Extend] (like [syn::Error] does)
    /// then the error can be pushed into an accumulator (such as a [MaybeError]
    /// in order to return the original value.
    pub fn push_unwrap(self, mut push_to: impl Extend<E>) -> T {
        let OkMaybe(ok, maybe) = self;
        if let Some(e) = maybe {
            push_to.extend(e);
        }

        ok
    }
}

/// Accumulate zero or more [syn::Error]-s
///
/// It is a best practice in proc macros to iterate over a collection of
/// results to accumulate as many errors as possible. Then at the end checking
/// if the collection contains any errors and converting them into a single
/// [syn::Error] (which can hold 1 or more errors) using [syn::Error::combine].
/// This struct makes that accumulation pattern easier.
///
/// FYI a [syn::Error] that contains multiple errors can be split apart using
/// [syn::Error::into_iter]. <https://github.com/dtolnay/syn/pull/1855>
///
/// Accumulate and return errors in a function:
///
/// ```rust
/// use proc_micro::MaybeError;
///
/// # #[allow(dead_code)]
/// fn my_fun() -> Result<(), syn::Error> {
///     # #[allow(unused_mut)]
///     let mut errors = MaybeError::new();
///     // ...
///
///     if let Some(error) = errors.maybe() {
///         Err(error)
///     } else {
///         Ok(())
///     }
/// }
/// ```
///
/// Convert a non-empty "maybe" error into a [syn::Error]:
///
/// ```
/// use proc_micro::MaybeError;
///
/// let mut errors = MaybeError::new();
///
/// assert!(!errors.has_err());
/// match syn::parse_str::<syn::Ident>("ident cannot hold string with spaces") {
///     Ok(_ident) => todo!(),
///     Err(error) => errors.push_back(error)
/// }
///
/// assert!(errors.has_err());
/// let error: syn::Error = errors.maybe().unwrap();
///
/// assert_eq!(
///     vec!["unexpected token".to_string()],
///     error
///         .into_iter()
///         .map(|e| format!("{e}"))
///         .collect::<Vec<_>>()
/// )
/// ```
#[derive(Debug, Default, Clone)]
pub struct MaybeError(VecDeque<syn::Error>);

impl MaybeError {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(error: syn::Error) -> Self {
        MaybeError(error.into_iter().collect::<VecDeque<syn::Error>>())
    }

    /// If an error exists, return it
    pub fn maybe(self) -> Option<syn::Error> {
        let mut errors = self.0;
        if let Some(mut error) = errors.pop_front() {
            for e in errors {
                error.combine(e);
            }
            Some(error)
        } else {
            None
        }
    }

    pub fn has_err(&self) -> bool {
        !self.0.is_empty()
    }

    /// Push a [syn::Error]
    pub fn push_back(&mut self, error: impl IntoIterator<Item = syn::Error>) {
        self.0.extend(error);
    }

    pub fn push_front(&mut self, error: syn::Error) {
        self.0.push_front(error);
    }
}

impl Deref for MaybeError {
    type Target = VecDeque<syn::Error>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MaybeError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Extend<syn::Error> for MaybeError {
    fn extend<T: IntoIterator<Item = syn::Error>>(&mut self, iter: T) {
        for e in iter {
            self.push_back(e);
        }
    }
}

impl Extend<syn::Error> for &mut MaybeError {
    fn extend<T: IntoIterator<Item = syn::Error>>(&mut self, iter: T) {
        for e in iter {
            self.push_back(e);
        }
    }
}
