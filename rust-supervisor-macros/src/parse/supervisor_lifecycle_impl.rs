//! Supervisor lifecycle implementation parser for role attribute macros.
//!
//! This module validates an inherent implementation block for the
//! `supervisor_role` attribute and records which lifecycle methods are present.

use proc_macro2::TokenStream;
use syn::{Error, ImplItem, ItemImpl, Result, Type};

/// Parsed supervisor lifecycle implementation block.
///
/// The parser preserves the original [`ItemImpl`] so generated code can emit
/// the user-authored inherent methods unchanged before adding trait bridges.
///
/// # Examples
///
/// ```rust,ignore
/// let parsed = parse_supervisor_lifecycle_impl(quote::quote! {
///     impl DemoSupervisor {
///         async fn build_tree(
///             &mut self,
///             ctx: &SupervisorContext,
///         ) -> SupervisorResult<SupervisorSpec> {
///             let _ = ctx;
///             Ok(SupervisorSpec::root(Vec::new()))
///         }
///     }
/// })?;
/// assert!(parsed.has_build_tree);
/// ```
pub(crate) struct SupervisorLifecycleImpl {
    /// Original inherent implementation block parsed from input tokens.
    pub(crate) item_impl: ItemImpl,
    /// Self type declared by the implementation block.
    pub(crate) self_ty: Box<Type>,
    /// Whether a `build_tree` lifecycle method is present.
    pub(crate) has_build_tree: bool,
    /// Whether a `run` lifecycle method is present.
    pub(crate) has_run: bool,
    /// Whether a `shutdown` lifecycle method is present.
    pub(crate) has_shutdown: bool,
}

/// Parses input tokens as a supervisor lifecycle implementation block.
///
/// # Arguments
///
/// - `tokens`: Token stream that must parse as a [`syn::ItemImpl`].
///
/// # Returns
///
/// Returns a [`SupervisorLifecycleImpl`] when the implementation block is
/// inherent and contains the required `build_tree` method.
///
/// # Errors
///
/// Returns [`syn::Error`] when parsing fails, the block is a trait
/// implementation, or `build_tree` is missing.
///
/// # Examples
///
/// ```rust,ignore
/// let parsed = parse_supervisor_lifecycle_impl(quote::quote! {
///     impl DemoSupervisor {
///         async fn build_tree(
///             &mut self,
///             ctx: &SupervisorContext,
///         ) -> SupervisorResult<SupervisorSpec> {
///             let _ = ctx;
///             Ok(SupervisorSpec::root(Vec::new()))
///         }
///     }
/// })?;
/// assert!(!parsed.has_shutdown);
/// ```
pub(crate) fn parse_supervisor_lifecycle_impl(
    tokens: TokenStream,
) -> Result<SupervisorLifecycleImpl> {
    parse_supervisor_lifecycle_item_impl(syn::parse2::<ItemImpl>(tokens)?)
}

/// Parses an existing [`syn::ItemImpl`] into supervisor lifecycle metadata.
///
/// # Arguments
///
/// - `impl_block`: Parsed implementation block from the attribute input.
///
/// # Returns
///
/// Returns lifecycle metadata with the original implementation block and method
/// presence flags.
///
/// # Errors
///
/// Returns [`syn::Error`] when the implementation block is not inherent or when
/// the required `build_tree` method is absent.
///
/// # Examples
///
/// ```rust,ignore
/// let item_impl: syn::ItemImpl = syn::parse_quote! {
///     impl DemoSupervisor {
///         async fn build_tree(
///             &mut self,
///             ctx: &SupervisorContext,
///         ) -> SupervisorResult<SupervisorSpec> {
///             let _ = ctx;
///             Ok(SupervisorSpec::root(Vec::new()))
///         }
///     }
/// };
/// let parsed = parse_supervisor_lifecycle_item_impl(item_impl)?;
/// assert_eq!(parsed.has_run, false);
/// ```
pub(crate) fn parse_supervisor_lifecycle_item_impl(
    impl_block: ItemImpl,
) -> Result<SupervisorLifecycleImpl> {
    if let Some((_, path, _)) = &impl_block.trait_ {
        return Err(Error::new_spanned(
            path,
            "`supervisor_role` must be attached to an inherent impl block",
        ));
    }

    let self_ty = impl_block.self_ty.clone();
    let mut has_build_tree = false;
    let mut has_run = false;
    let mut has_shutdown = false;

    for item in &impl_block.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        if method.sig.ident == "build_tree" {
            has_build_tree = true;
        } else if method.sig.ident == "run" {
            has_run = true;
        } else if method.sig.ident == "shutdown" {
            has_shutdown = true;
        }
    }

    if !has_build_tree {
        return Err(Error::new_spanned(
            &impl_block.self_ty,
            "missing required supervisor lifecycle method `build_tree`",
        ));
    }

    Ok(SupervisorLifecycleImpl {
        item_impl: impl_block,
        self_ty,
        has_build_tree,
        has_run,
        has_shutdown,
    })
}
