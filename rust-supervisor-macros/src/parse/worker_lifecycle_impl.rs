//! Worker lifecycle implementation parser for role attribute macros.
//!
//! This module converts an inherent implementation block into a small worker
//! lifecycle model. Expansion modules can keep the original `ItemImpl` while
//! checking whether required and optional lifecycle methods are present.

use proc_macro2::TokenStream;
use syn::{Error, ImplItem, ItemImpl, Result, Type};

/// Parsed worker lifecycle implementation block.
///
/// The model preserves the original [`ItemImpl`] so expansion code can reuse
/// the user-authored methods without reparsing the annotated item.
///
/// # Examples
///
/// ```rust,ignore
/// let parsed = parse_worker_lifecycle_impl(quote::quote! {
///     impl DemoWorker {
///         async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
///             Ok(())
///         }
///     }
/// })?;
/// assert!(parsed.has_work);
/// ```
pub(crate) struct WorkerLifecycleImpl {
    /// Original implementation block parsed from input tokens.
    pub(crate) item_impl: ItemImpl,
    /// Self type declared by the implementation block.
    pub(crate) self_ty: Box<Type>,
    /// Whether an `init` lifecycle method is present.
    pub(crate) has_init: bool,
    /// Whether a `work` lifecycle method is present.
    pub(crate) has_work: bool,
    /// Whether a `complete` lifecycle method is present.
    pub(crate) has_complete: bool,
}

/// Parses input tokens as a worker lifecycle implementation block.
///
/// # Parameters
///
/// - `tokens`: Token stream that must parse as a [`syn::ItemImpl`].
///
/// # Returns
///
/// Returns a [`WorkerLifecycleImpl`] when the implementation block is inherent
/// and contains a `work` method. Returns [`syn::Error`] when parsing fails,
/// when the input is a trait implementation, or when `work` is missing.
///
/// # Examples
///
/// ```rust,ignore
/// let parsed = parse_worker_lifecycle_impl(quote::quote! {
///     impl DemoWorker {
///         async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
///             Ok(())
///         }
///     }
/// })?;
/// assert!(!parsed.has_complete);
/// ```
pub(crate) fn parse_worker_lifecycle_impl(tokens: TokenStream) -> Result<WorkerLifecycleImpl> {
    parse_worker_lifecycle_item_impl(syn::parse2::<ItemImpl>(tokens)?)
}

/// Parses an existing [`syn::ItemImpl`] into a worker lifecycle model.
///
/// # Parameters
///
/// - `impl_block`: Parsed implementation block from the attribute input.
///
/// # Returns
///
/// Returns a [`WorkerLifecycleImpl`] with lifecycle method flags. Returns
/// [`syn::Error`] if the implementation block is not inherent or the required
/// `work` method is missing.
///
/// # Examples
///
/// ```rust,ignore
/// let item_impl: syn::ItemImpl = syn::parse_quote! {
///     impl DemoWorker {
///         async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
///             Ok(())
///         }
///     }
/// };
/// let parsed = parse_worker_lifecycle_item_impl(item_impl)?;
/// assert_eq!(parsed.has_init, false);
/// ```
pub(crate) fn parse_worker_lifecycle_item_impl(
    impl_block: ItemImpl,
) -> Result<WorkerLifecycleImpl> {
    if impl_block.trait_.is_some() {
        return Err(Error::new_spanned(
            &impl_block.self_ty,
            "`#[worker]` requires an inherent impl block",
        ));
    }

    let self_ty = impl_block.self_ty.clone();
    let mut has_init = false;
    let mut has_work = false;
    let mut has_complete = false;

    for item in &impl_block.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        if method.sig.ident == "init" {
            has_init = true;
        } else if method.sig.ident == "work" {
            has_work = true;
        } else if method.sig.ident == "complete" {
            has_complete = true;
        }
    }

    if !has_work {
        return Err(Error::new_spanned(
            &impl_block.self_ty,
            "missing required lifecycle method `work`",
        ));
    }

    Ok(WorkerLifecycleImpl {
        item_impl: impl_block,
        self_ty,
        has_init,
        has_work,
        has_complete,
    })
}
