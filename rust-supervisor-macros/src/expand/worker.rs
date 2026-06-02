//! Worker role code generation.
//!
//! The generated code targets explicit runtime contracts in `rust_supervisor`
//! and keeps user-authored lifecycle methods in their original inherent impl.

use crate::parse::role_args::RoleArgs;
use crate::parse::worker_lifecycle_impl::WorkerLifecycleImpl;
use proc_macro2::TokenStream;
use quote::quote;

/// Expands a parsed worker role into runtime bridge code.
///
/// # Arguments
///
/// - `args`: Parsed worker metadata from the attribute arguments.
/// - `lifecycle`: Parsed inherent impl block with worker lifecycle metadata.
///
/// # Returns
///
/// Returns generated Rust tokens that include the original impl block, a
/// `WorkerRole` implementation, and worker helper methods.
///
/// # Examples
///
/// ```rust,ignore
/// let tokens = expand_worker(&args, &lifecycle);
/// assert!(!tokens.is_empty());
/// ```
pub(crate) fn expand_worker(args: &RoleArgs, lifecycle: &WorkerLifecycleImpl) -> TokenStream {
    debug_assert!(lifecycle.has_work);

    let original_impl = &lifecycle.item_impl;
    let self_ty = lifecycle.self_ty.as_ref();
    let id = &args.id;
    let name = &args.name;
    let init_impl = optional_init_method(lifecycle);
    let complete_impl = optional_complete_method(lifecycle);

    quote! {
        #original_impl

        impl ::rust_supervisor::role::traits::worker::WorkerRole for #self_ty {
            #init_impl

            fn work<'a>(
                &'a mut self,
                ctx: &'a ::rust_supervisor::role::context::worker::WorkerContext,
            ) -> impl ::std::future::Future<
                Output = ::rust_supervisor::role::result::worker::WorkerResult<()>,
            > + Send + 'a {
                async move {
                    self.work(ctx).await
                }
            }

            #complete_impl
        }

        impl #self_ty {
            pub fn worker_adapter(
                self,
            ) -> ::rust_supervisor::role::adapter::worker::WorkerRoleAdapter<Self> {
                ::rust_supervisor::role::adapter::worker::WorkerRoleAdapter::new(self)
            }

            pub fn child_spec(
                self,
            ) -> ::std::result::Result<
                ::rust_supervisor::spec::child::ChildSpec,
                ::rust_supervisor::error::types::SupervisorError,
            > {
                ::rust_supervisor::spec::child_builder::ChildSpecBuilder::worker(
                    ::rust_supervisor::id::types::ChildId::new(#id),
                    #name,
                    ::rust_supervisor::spec::child::TaskKind::AsyncWorker,
                    ::std::sync::Arc::new(self.worker_adapter()),
                )
                .build()
            }
        }
    }
}

/// Generates an optional `init` lifecycle bridge.
///
/// # Arguments
///
/// - `lifecycle`: Parsed lifecycle metadata for the annotated impl block.
///
/// # Returns
///
/// Returns tokens that override `WorkerRole::init` only when the user defined
/// an inherent `init` method.
fn optional_init_method(lifecycle: &WorkerLifecycleImpl) -> TokenStream {
    if !lifecycle.has_init {
        return TokenStream::new();
    }
    quote! {
        fn init<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::worker::WorkerContext,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::worker::WorkerResult<()>,
        > + Send + 'a {
            async move {
                self.init(ctx).await
            }
        }
    }
}

/// Generates an optional `complete` lifecycle bridge.
///
/// # Arguments
///
/// - `lifecycle`: Parsed lifecycle metadata for the annotated impl block.
///
/// # Returns
///
/// Returns tokens that override `WorkerRole::complete` only when the user
/// defined an inherent `complete` method.
fn optional_complete_method(lifecycle: &WorkerLifecycleImpl) -> TokenStream {
    if !lifecycle.has_complete {
        return TokenStream::new();
    }
    quote! {
        fn complete<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::worker::WorkerContext,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::worker::WorkerResult<()>,
        > + Send + 'a {
            async move {
                self.complete(ctx).await
            }
        }
    }
}
