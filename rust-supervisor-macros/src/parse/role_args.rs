//! Parsers for role attribute metadata.
//!
//! This module owns the argument parsing shape for role attribute macros.

use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token, parenthesized};

/// Parsed common arguments for the `service` attribute macro.
///
/// The parser accepts both `id = "..."` and `id("...")` field syntax.
///
/// # Examples
///
/// ```ignore
/// let args = parse_role_args(quote::quote! {
///     id = "quote-service",
///     name = "Quote Service"
/// })?;
///
/// assert_eq!(args.id.value(), "quote-service");
/// assert_eq!(args.name.value(), "Quote Service");
/// # Ok::<(), syn::Error>(())
/// ```
pub(crate) struct RoleArgs {
    /// Service identifier string literal.
    pub(crate) id: LitStr,
    /// Service display name string literal.
    pub(crate) name: LitStr,
}

impl Parse for RoleArgs {
    /// Parses service role arguments from a syn parse stream.
    ///
    /// # Arguments
    ///
    /// - `input`: Token stream reader positioned at the attribute arguments.
    ///
    /// # Returns
    ///
    /// Returns parsed service metadata with required `id` and `name` literals.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] when a field is missing, duplicated, unknown, or
    /// not backed by a string literal value.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        parse_role_args_stream(input)
    }
}

/// Parses `#[service(...)]` arguments from a token stream.
///
/// # Arguments
///
/// - `tokens`: Raw attribute argument tokens supplied by the proc macro entry.
///
/// # Returns
///
/// Returns [`RoleArgs`] with the required `id` and `name` fields.
///
/// # Errors
///
/// Returns [`syn::Error`] for duplicate fields, unknown fields, missing fields,
/// malformed field syntax, or non-string literal values.
///
/// # Examples
///
/// ```ignore
/// let parsed = parse_role_args(quote::quote! {
///     id("quote-service"),
///     name("Quote Service")
/// })?;
///
/// assert_eq!(parsed.id.value(), "quote-service");
/// assert_eq!(parsed.name.value(), "Quote Service");
/// # Ok::<(), syn::Error>(())
/// ```
pub(crate) fn parse_role_args(tokens: TokenStream) -> Result<RoleArgs> {
    syn::parse2(tokens)
}

/// Parses `#[service(...)]` arguments from an existing syn parse stream.
///
/// # Arguments
///
/// - `input`: Parse stream that contains only the service attribute arguments.
///
/// # Returns
///
/// Returns [`RoleArgs`] after validating all required fields.
///
/// # Errors
///
/// Returns [`syn::Error`] for unknown fields, duplicates, missing values, or
/// unsupported literal types.
pub(crate) fn parse_role_args_stream(input: ParseStream<'_>) -> Result<RoleArgs> {
    let mut id = None;
    let mut name = None;

    while !input.is_empty() {
        let key = input.parse::<Ident>()?;

        if key == "id" {
            let value = parse_string_field(input, &key)?;
            set_once(&mut id, &key, value)?;
        } else if key == "name" {
            let value = parse_string_field(input, &key)?;
            set_once(&mut name, &key, value)?;
        } else {
            return Err(Error::new(
                key.span(),
                format!(
                    "unknown service argument `{}`; expected `id` or `name`",
                    key
                ),
            ));
        }

        if input.is_empty() {
            break;
        }

        input.parse::<Token![,]>()?;
    }

    let id = id.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            "missing required service argument `id`",
        )
    })?;
    let name = name.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            "missing required service argument `name`",
        )
    })?;

    Ok(RoleArgs { id, name })
}

/// Parses a supported string field value syntax.
///
/// # Arguments
///
/// - `input`: Parse stream positioned after the field key.
/// - `key`: Field identifier used in the error message.
///
/// # Returns
///
/// Returns the parsed [`LitStr`] for `key = "..."` or `key("...")`.
///
/// # Errors
///
/// Returns [`syn::Error`] when the field uses unsupported syntax or does not
/// contain exactly one string literal.
fn parse_string_field(input: ParseStream<'_>, key: &Ident) -> Result<LitStr> {
    if input.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        return parse_string_literal(input, key);
    }

    if input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);

        let value = parse_string_literal(&content, key)?;
        if !content.is_empty() {
            return Err(Error::new(
                key.span(),
                format!(
                    "service argument `{}` accepts exactly one string literal",
                    key
                ),
            ));
        }

        return Ok(value);
    }

    Err(Error::new(
        key.span(),
        format!(
            "service argument `{}` must use `{}` = \"...\" or `{}(\"...\")`",
            key, key, key
        ),
    ))
}

/// Parses a string literal value.
///
/// # Arguments
///
/// - `input`: Parse stream positioned at the expected string literal.
/// - `key`: Field identifier used in the error message.
///
/// # Returns
///
/// Returns the parsed [`LitStr`].
///
/// # Errors
///
/// Returns [`syn::Error`] when the next token is not a string literal.
fn parse_string_literal(input: ParseStream<'_>, key: &Ident) -> Result<LitStr> {
    if input.peek(LitStr) {
        return input.parse::<LitStr>();
    }

    Err(input.error(format!(
        "service argument `{}` requires a string literal",
        key
    )))
}

/// Stores one parsed field and rejects duplicates.
///
/// # Arguments
///
/// - `slot`: Destination slot for the parsed value.
/// - `key`: Field identifier used in the duplicate error.
/// - `value`: Parsed string literal value.
///
/// # Returns
///
/// Returns `Ok(())` after storing the value.
///
/// # Errors
///
/// Returns [`syn::Error`] when the destination slot already has a value.
fn set_once(slot: &mut Option<LitStr>, key: &Ident, value: LitStr) -> Result<()> {
    if slot.is_some() {
        return Err(Error::new(
            key.span(),
            format!("duplicate service argument `{}`", key),
        ));
    }

    *slot = Some(value);
    Ok(())
}
