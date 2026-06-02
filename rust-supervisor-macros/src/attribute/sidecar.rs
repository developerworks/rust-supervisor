//! Attribute entry for the sidecar role macro.
//!
//! This module keeps the procedural macro boundary thin and delegates syntax
//! parsing plus generated code construction to dedicated modules.

use crate::expand::sidecar::expand_sidecar;
use crate::parse::sidecar_args::parse_sidecar_args;
use crate::parse::sidecar_lifecycle_impl::parse_sidecar_lifecycle_impl;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

/// Expands the `#[sidecar]` attribute macro.
///
/// # Arguments
///
/// - `args`: Attribute arguments containing sidecar metadata.
/// - `item`: Inherent impl block that owns sidecar lifecycle methods.
///
/// # Returns
///
/// Returns generated tokens for the original impl block, the role trait bridge,
/// and sidecar helper methods. Syntax errors are converted into compiler
/// diagnostics.
pub(crate) fn sidecar(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let item = TokenStream2::from(item);
    expand_sidecar_attribute(args, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Builds sidecar macro output after converting token stream ownership.
///
/// # Arguments
///
/// - `args`: Attribute arguments as proc macro compatible tokens.
/// - `item`: Inherent impl block tokens from the annotated item.
///
/// # Returns
///
/// Returns generated Rust tokens or a syntax error produced by parsing.
fn expand_sidecar_attribute(args: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let sidecar_args = parse_sidecar_args(args)?;
    let lifecycle_impl = parse_sidecar_lifecycle_impl(item)?;
    Ok(expand_sidecar(&sidecar_args, &lifecycle_impl))
}
