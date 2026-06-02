//! Parsers for sidecar attribute metadata.
//!
//! This module owns argument parsing for the `#[sidecar]` attribute macro.

use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token, parenthesized};

/// Parsed arguments for the `sidecar` attribute macro.
///
/// The parser accepts `id = "..."`, `name = "..."`, and `primary = "..."`
/// field syntax. It also accepts the same parenthesized shape used by the
/// service parser for consistency.
///
/// # Examples
///
/// ```ignore
/// let args = parse_sidecar_args(quote::quote! {
///     id = "metrics-sidecar",
///     name = "Metrics Sidecar",
///     primary = "quote-service"
/// })?;
///
/// assert_eq!(args.id.value(), "metrics-sidecar");
/// assert_eq!(args.primary.value(), "quote-service");
/// # Ok::<(), syn::Error>(())
/// ```
pub(crate) struct SidecarArgs {
    /// Sidecar child identifier string literal.
    pub(crate) id: LitStr,
    /// Sidecar display name string literal.
    pub(crate) name: LitStr,
    /// Primary child identifier string literal.
    pub(crate) primary: LitStr,
}

impl Parse for SidecarArgs {
    /// Parses sidecar role arguments from a syn parse stream.
    ///
    /// # Arguments
    ///
    /// - `input`: Token stream reader positioned at the attribute arguments.
    ///
    /// # Returns
    ///
    /// Returns parsed sidecar metadata with required `id`, `name`, and
    /// `primary` literals.
    ///
    /// # Errors
    ///
    /// Returns [`syn::Error`] when a field is missing, duplicated, unknown, or
    /// not backed by a string literal value.
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        parse_sidecar_args_stream(input)
    }
}

/// Parses `#[sidecar(...)]` arguments from a token stream.
///
/// # Arguments
///
/// - `tokens`: Raw attribute argument tokens supplied by the proc macro entry.
///
/// # Returns
///
/// Returns [`SidecarArgs`] with the required `id`, `name`, and `primary`
/// fields.
///
/// # Errors
///
/// Returns [`syn::Error`] for duplicate fields, unknown fields, missing fields,
/// malformed field syntax, or non-string literal values.
///
/// # Examples
///
/// ```ignore
/// let parsed = parse_sidecar_args(quote::quote! {
///     id("metrics-sidecar"),
///     name("Metrics Sidecar"),
///     primary("quote-service")
/// })?;
///
/// assert_eq!(parsed.name.value(), "Metrics Sidecar");
/// assert_eq!(parsed.primary.value(), "quote-service");
/// # Ok::<(), syn::Error>(())
/// ```
pub(crate) fn parse_sidecar_args(tokens: TokenStream) -> Result<SidecarArgs> {
    syn::parse2(tokens)
}

/// Parses `#[sidecar(...)]` arguments from an existing syn parse stream.
///
/// # Arguments
///
/// - `input`: Parse stream that contains only the sidecar attribute arguments.
///
/// # Returns
///
/// Returns [`SidecarArgs`] after validating all required fields.
///
/// # Errors
///
/// Returns [`syn::Error`] for unknown fields, duplicates, missing values, or
/// unsupported literal types.
pub(crate) fn parse_sidecar_args_stream(input: ParseStream<'_>) -> Result<SidecarArgs> {
    let mut id = None;
    let mut name = None;
    let mut primary = None;

    while !input.is_empty() {
        let key = input.parse::<Ident>()?;

        if key == "id" {
            let value = parse_string_field(input, &key)?;
            set_once(&mut id, &key, value)?;
        } else if key == "name" {
            let value = parse_string_field(input, &key)?;
            set_once(&mut name, &key, value)?;
        } else if key == "primary" {
            let value = parse_string_field(input, &key)?;
            set_once(&mut primary, &key, value)?;
        } else {
            return Err(Error::new(
                key.span(),
                format!(
                    "unknown sidecar argument `{}`; expected `id`, `name`, or `primary`",
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
            "missing required sidecar argument `id`",
        )
    })?;
    let name = name.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            "missing required sidecar argument `name`",
        )
    })?;
    let primary = primary.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            "missing required sidecar argument `primary`",
        )
    })?;

    Ok(SidecarArgs { id, name, primary })
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
                    "sidecar argument `{}` accepts exactly one string literal",
                    key
                ),
            ));
        }

        return Ok(value);
    }

    Err(Error::new(
        key.span(),
        format!(
            "sidecar argument `{}` must use `{}` = \"...\" or `{}(\"...\")`",
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
        "sidecar argument `{}` requires a string literal",
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
            format!("duplicate sidecar argument `{}`", key),
        ));
    }

    *slot = Some(value);
    Ok(())
}
