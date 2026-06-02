//! Attribute entry for the worker role macro.
//!
//! This module keeps the procedural macro boundary thin and delegates syntax
//! parsing plus generated code construction to dedicated modules.

use crate::expand::worker::expand_worker;
use crate::parse::role_args::parse_role_args;
use crate::parse::worker_lifecycle_impl::parse_worker_lifecycle_impl;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

/// Expands the `#[worker]` attribute macro.
///
/// # Arguments
///
/// - `args`: Attribute arguments containing worker metadata.
/// - `item`: Inherent impl block that owns worker lifecycle methods.
///
/// # Returns
///
/// Returns generated tokens for the original impl block, the role trait bridge,
/// and worker helper methods. Syntax errors are converted into compiler
/// diagnostics.
///
/// # Examples
///
/// ```rust,ignore
/// let output = worker(args, item);
/// assert!(!output.is_empty());
/// ```
pub(crate) fn worker(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let item = TokenStream2::from(item);
    expand_worker_attribute(args, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Builds worker macro output after converting token stream ownership.
///
/// # Arguments
///
/// - `args`: Attribute arguments as proc macro compatible tokens.
/// - `item`: Inherent impl block tokens from the annotated item.
///
/// # Returns
///
/// Returns generated Rust tokens or a syntax error produced by parsing.
///
/// # Examples
///
/// ```rust,ignore
/// let tokens = expand_worker_attribute(args, item)?;
/// assert!(!tokens.is_empty());
/// # Ok::<(), syn::Error>(())
/// ```
fn expand_worker_attribute(args: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let role_args = parse_role_args(args)?;
    let lifecycle_impl = parse_worker_lifecycle_impl(item)?;
    Ok(expand_worker(&role_args, &lifecycle_impl))
}
