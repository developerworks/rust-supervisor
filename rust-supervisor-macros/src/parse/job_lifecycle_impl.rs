//! Job lifecycle implementation parser for role attribute macros.
//!
//! This module converts an inherent implementation block into a compact job
//! lifecycle model. Expansion code can reuse the original `ItemImpl` while
//! checking whether required and optional job methods are present.

use proc_macro2::TokenStream;
use syn::{Error, ImplItem, ItemImpl, Result, Type};

/// Parsed job lifecycle implementation block.
///
/// The model preserves the original [`ItemImpl`] so expansion code can keep
/// generics, predicates, attributes, and method bodies intact.
///
/// # Examples
///
/// ```rust,ignore
/// let parsed = parse_job_lifecycle_impl(quote::quote! {
///     impl DailyReportJob {
///         async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
///             Ok(())
///         }
///     }
/// })?;
/// assert!(parsed.has_run);
/// ```
pub(crate) struct JobLifecycleImpl {
    /// Original implementation block parsed from input tokens.
    pub(crate) item_impl: ItemImpl,
    /// Self type declared by the implementation block.
    pub(crate) self_ty: Box<Type>,
    /// Whether an `init` lifecycle method is present.
    pub(crate) has_init: bool,
    /// Whether a `run` lifecycle method is present.
    pub(crate) has_run: bool,
    /// Whether a `complete` lifecycle method is present.
    pub(crate) has_complete: bool,
}

/// Parses input tokens as a job lifecycle implementation block.
///
/// # Arguments
///
/// - `tokens`: Token stream that must parse as a [`syn::ItemImpl`].
///
/// # Returns
///
/// Returns a [`JobLifecycleImpl`] when the implementation block is inherent and
/// contains a `run` method. Returns [`syn::Error`] when parsing fails, the impl
/// block is not inherent, or the required method is missing.
///
/// # Examples
///
/// ```rust,ignore
/// let parsed = parse_job_lifecycle_impl(quote::quote! {
///     impl DailyReportJob {
///         async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
///             Ok(())
///         }
///     }
/// })?;
/// assert!(!parsed.has_complete);
/// ```
pub(crate) fn parse_job_lifecycle_impl(tokens: TokenStream) -> Result<JobLifecycleImpl> {
    parse_job_lifecycle_item_impl(syn::parse2::<ItemImpl>(tokens)?)
}

/// Parses an existing [`syn::ItemImpl`] into a job lifecycle model.
///
/// # Arguments
///
/// - `impl_block`: Parsed implementation block from the attribute input.
///
/// # Returns
///
/// Returns a [`JobLifecycleImpl`] with lifecycle method flags. Returns
/// [`syn::Error`] if the input is not an inherent impl block or the required
/// `run` method is missing.
///
/// # Examples
///
/// ```rust,ignore
/// let item_impl: syn::ItemImpl = syn::parse_quote! {
///     impl DailyReportJob {
///         async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
///             Ok(())
///         }
///     }
/// };
/// let parsed = parse_job_lifecycle_item_impl(item_impl)?;
/// assert_eq!(parsed.has_init, false);
/// ```
pub(crate) fn parse_job_lifecycle_item_impl(impl_block: ItemImpl) -> Result<JobLifecycleImpl> {
    if impl_block.trait_.is_some() {
        return Err(Error::new_spanned(
            &impl_block.self_ty,
            "job attribute requires an inherent impl block",
        ));
    }

    let self_ty = impl_block.self_ty.clone();
    let mut has_init = false;
    let mut has_run = false;
    let mut has_complete = false;

    for item in &impl_block.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        if method.sig.ident == "init" {
            has_init = true;
        } else if method.sig.ident == "run" {
            has_run = true;
        } else if method.sig.ident == "complete" {
            has_complete = true;
        }
    }

    if !has_run {
        return Err(Error::new_spanned(
            &impl_block.self_ty,
            "missing required job lifecycle method `run`",
        ));
    }

    Ok(JobLifecycleImpl {
        item_impl: impl_block,
        self_ty,
        has_init,
        has_run,
        has_complete,
    })
}
