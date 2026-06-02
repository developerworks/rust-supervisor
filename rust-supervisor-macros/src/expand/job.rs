//! Job role code generation.
//!
//! The generated code targets explicit runtime contracts in `rust_supervisor`
//! and keeps user-authored lifecycle methods in their original inherent impl.

use crate::parse::job_lifecycle_impl::JobLifecycleImpl;
use crate::parse::role_args::RoleArgs;
use proc_macro2::TokenStream;
use quote::quote;

/// Expands a parsed job role into runtime bridge code.
///
/// # Arguments
///
/// - `args`: Parsed job metadata from the attribute arguments.
/// - `lifecycle`: Parsed inherent impl block with lifecycle method metadata.
///
/// # Returns
///
/// Returns generated Rust tokens that include the original impl block, a
/// `JobRole` implementation, and job helper methods.
pub(crate) fn expand_job(args: &RoleArgs, lifecycle: &JobLifecycleImpl) -> TokenStream {
    debug_assert!(lifecycle.has_run);

    let original_impl = &lifecycle.item_impl;
    let self_ty = lifecycle.self_ty.as_ref();
    let id = &args.id;
    let name = &args.name;
    let init_impl = optional_init_method(lifecycle);
    let complete_impl = optional_complete_method(lifecycle);
    let (impl_generics, _ty_generics, where_clause) = lifecycle.item_impl.generics.split_for_impl();

    quote! {
        #original_impl

        impl #impl_generics ::rust_supervisor::role::traits::job::JobRole for #self_ty #where_clause {
            #init_impl

            fn run<'a>(
                &'a mut self,
                ctx: &'a ::rust_supervisor::role::context::job::JobContext,
            ) -> impl ::std::future::Future<
                Output = ::rust_supervisor::role::result::job::JobResult<()>,
            > + Send + 'a {
                async move {
                    self.run(ctx).await
                }
            }

            #complete_impl
        }

        impl #impl_generics #self_ty #where_clause {
            pub fn job_adapter(
                self,
            ) -> ::rust_supervisor::role::adapter::job::JobRoleAdapter<Self> {
                ::rust_supervisor::role::adapter::job::JobRoleAdapter::new(self)
            }

            pub fn child_spec(
                self,
            ) -> ::std::result::Result<
                ::rust_supervisor::spec::child::ChildSpec,
                ::rust_supervisor::error::types::SupervisorError,
            > {
                ::rust_supervisor::spec::child_builder::ChildSpecBuilder::job(
                    ::rust_supervisor::id::types::ChildId::new(#id),
                    #name,
                    ::rust_supervisor::spec::child::TaskKind::AsyncWorker,
                    ::std::sync::Arc::new(self.job_adapter()),
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
/// Returns tokens that override `JobRole::init` only when the user defined an
/// inherent `init` method.
fn optional_init_method(lifecycle: &JobLifecycleImpl) -> TokenStream {
    if !lifecycle.has_init {
        return TokenStream::new();
    }
    quote! {
        fn init<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::job::JobContext,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::job::JobResult<()>,
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
/// Returns tokens that override `JobRole::complete` only when the user defined
/// an inherent `complete` method.
fn optional_complete_method(lifecycle: &JobLifecycleImpl) -> TokenStream {
    if !lifecycle.has_complete {
        return TokenStream::new();
    }
    quote! {
        fn complete<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::job::JobContext,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::job::JobResult<()>,
        > + Send + 'a {
            async move {
                self.complete(ctx).await
            }
        }
    }
}
