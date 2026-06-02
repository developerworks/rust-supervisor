//! Attribute entry for the job role macro.
//!
//! This module keeps the procedural macro boundary thin and delegates syntax
//! parsing plus generated code construction to dedicated modules.

use crate::expand::job::expand_job;
use crate::parse::job_lifecycle_impl::parse_job_lifecycle_impl;
use crate::parse::role_args::parse_role_args;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

/// Expands the `#[job]` attribute macro.
///
/// # Arguments
///
/// - `args`: Attribute arguments containing job metadata.
/// - `item`: Inherent impl block that owns job lifecycle methods.
///
/// # Returns
///
/// Returns generated tokens for the original impl block, the role trait bridge,
/// and job helper methods. Syntax errors are converted into compiler
/// diagnostics.
pub(crate) fn job(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    let item = TokenStream2::from(item);
    expand_job_attribute(args, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Builds job macro output after converting token stream ownership.
///
/// # Arguments
///
/// - `args`: Attribute arguments as proc macro compatible tokens.
/// - `item`: Inherent impl block tokens from the annotated item.
///
/// # Returns
///
/// Returns generated Rust tokens or a syntax error produced by parsing.
fn expand_job_attribute(args: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let role_args = parse_role_args(args)?;
    let lifecycle_impl = parse_job_lifecycle_impl(item)?;
    Ok(expand_job(&role_args, &lifecycle_impl))
}
