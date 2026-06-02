//! Attribute entry for the supervisor role macro.
//!
//! This module keeps the procedural macro boundary thin and delegates argument
//! parsing, lifecycle validation, and generated code construction.

use crate::expand::supervisor_role::expand_supervisor_role;
use crate::parse::role_args::parse_role_args;
use crate::parse::supervisor_lifecycle_impl::parse_supervisor_lifecycle_impl;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

/// Expands the `#[supervisor_role]` attribute macro.
///
/// # Arguments
///
/// - `args`: Attribute arguments containing supervisor role metadata.
/// - `item`: Inherent implementation block that owns lifecycle methods.
///
/// # Returns
///
/// Returns generated tokens for the original implementation block, the
/// supervisor role trait bridge, and helper methods. Syntax errors are
/// converted into compiler diagnostics.
///
/// # Examples
///
/// ```rust,ignore
/// let output = supervisor_role(args, item);
/// assert!(!output.is_empty());
/// ```
pub(crate) fn supervisor_role(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let item = TokenStream2::from(item);
    expand_supervisor_role_attribute(args, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Builds supervisor role macro output after converting token ownership.
///
/// # Arguments
///
/// - `args`: Attribute arguments as proc macro compatible tokens.
/// - `item`: Inherent implementation block tokens from the annotated item.
///
/// # Returns
///
/// Returns generated Rust tokens or a syntax error produced by parsing.
fn expand_supervisor_role_attribute(
    args: TokenStream2,
    item: TokenStream2,
) -> syn::Result<TokenStream2> {
    let role_args = parse_role_args(args)?;
    let lifecycle_impl = parse_supervisor_lifecycle_impl(item)?;
    Ok(expand_supervisor_role(&role_args, &lifecycle_impl))
}
