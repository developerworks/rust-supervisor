//! Lifecycle implementation parser for the sidecar attribute macro.
//!
//! This module converts an inherent implementation block into a sidecar
//! lifecycle model while preserving the original item for expansion.

use proc_macro2::TokenStream;
use syn::{Error, ImplItem, ItemImpl, Result, Type};

/// Parsed sidecar lifecycle implementation block.
///
/// The model preserves the original [`ItemImpl`] so expansion code can reuse
/// generics, predicates, attributes, and method bodies without reparsing.
///
/// # Examples
///
/// ```rust,ignore
/// let parsed = parse_sidecar_lifecycle_impl(quote::quote! {
///     impl MetricsSidecar {
///         async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
///             Ok(())
///         }
///     }
/// })?;
/// assert!(parsed.has_run);
/// ```
pub(crate) struct SidecarLifecycleImpl {
    /// Original implementation block parsed from input tokens.
    pub(crate) item_impl: ItemImpl,
    /// Self type declared by the implementation block.
    pub(crate) self_ty: Box<Type>,
    /// Whether an `init` lifecycle method is present.
    pub(crate) has_init: bool,
    /// Whether a `run` lifecycle method is present.
    pub(crate) has_run: bool,
    /// Whether a `shutdown` lifecycle method is present.
    pub(crate) has_shutdown: bool,
}

/// Parses input tokens as a sidecar lifecycle implementation block.
///
/// # Arguments
///
/// - `tokens`: Token stream that must parse as a [`syn::ItemImpl`].
///
/// # Returns
///
/// Returns a [`SidecarLifecycleImpl`] when the implementation block is
/// inherent and contains a `run` method. Returns [`syn::Error`] when parsing
/// fails, the block implements a trait, or the required method is missing.
///
/// # Examples
///
/// ```rust,ignore
/// let parsed = parse_sidecar_lifecycle_impl(quote::quote! {
///     impl MetricsSidecar {
///         async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
///             Ok(())
///         }
///     }
/// })?;
/// assert!(!parsed.has_shutdown);
/// ```
pub(crate) fn parse_sidecar_lifecycle_impl(tokens: TokenStream) -> Result<SidecarLifecycleImpl> {
    parse_sidecar_lifecycle_item_impl(syn::parse2::<ItemImpl>(tokens)?)
}

/// Parses an existing [`syn::ItemImpl`] into a sidecar lifecycle model.
///
/// # Arguments
///
/// - `impl_block`: Parsed implementation block from the attribute input.
///
/// # Returns
///
/// Returns a [`SidecarLifecycleImpl`] with lifecycle method flags. Returns
/// [`syn::Error`] if the block is not inherent or the required `run` method is
/// missing.
///
/// # Examples
///
/// ```rust,ignore
/// let item_impl: syn::ItemImpl = syn::parse_quote! {
///     impl MetricsSidecar {
///         async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
///             Ok(())
///         }
///     }
/// };
/// let parsed = parse_sidecar_lifecycle_item_impl(item_impl)?;
/// assert_eq!(parsed.has_init, false);
/// ```
pub(crate) fn parse_sidecar_lifecycle_item_impl(
    impl_block: ItemImpl,
) -> Result<SidecarLifecycleImpl> {
    if let Some((_, trait_path, _)) = &impl_block.trait_ {
        return Err(Error::new_spanned(
            trait_path,
            "sidecar attribute must be applied to an inherent impl block",
        ));
    }

    let self_ty = impl_block.self_ty.clone();
    let mut has_init = false;
    let mut has_run = false;
    let mut has_shutdown = false;

    for item in &impl_block.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        if method.sig.ident == "init" {
            has_init = true;
        } else if method.sig.ident == "run" {
            has_run = true;
        } else if method.sig.ident == "shutdown" {
            has_shutdown = true;
        }
    }

    if !has_run {
        return Err(Error::new_spanned(
            &impl_block.self_ty,
            "missing required sidecar lifecycle method `run`",
        ));
    }

    Ok(SidecarLifecycleImpl {
        item_impl: impl_block,
        self_ty,
        has_init,
        has_run,
        has_shutdown,
    })
}
