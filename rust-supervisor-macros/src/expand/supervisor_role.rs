//! Supervisor role code generation.
//!
//! The generated code bridges user-authored supervisor lifecycle methods into
//! runtime contracts while keeping the original inherent implementation block.

use crate::parse::role_args::RoleArgs;
use crate::parse::supervisor_lifecycle_impl::SupervisorLifecycleImpl;
use proc_macro2::TokenStream;
use quote::quote;

/// Expands a parsed supervisor role into runtime bridge code.
///
/// # Arguments
///
/// - `args`: Parsed supervisor metadata from the attribute arguments.
/// - `lifecycle`: Parsed inherent implementation block with lifecycle flags.
///
/// # Returns
///
/// Returns generated Rust tokens that include the original implementation
/// block, a `SupervisorRole` implementation, and supervisor helper methods.
///
/// # Examples
///
/// ```rust,ignore
/// let tokens = expand_supervisor_role(&args, &lifecycle);
/// assert!(!tokens.is_empty());
/// ```
pub(crate) fn expand_supervisor_role(
    args: &RoleArgs,
    lifecycle: &SupervisorLifecycleImpl,
) -> TokenStream {
    debug_assert!(lifecycle.has_build_tree);

    let original_impl = &lifecycle.item_impl;
    let self_ty = lifecycle.self_ty.as_ref();
    let generics = &lifecycle.item_impl.generics;
    let (impl_generics, _ty_generics, where_clause) = generics.split_for_impl();
    let id = &args.id;
    let name = &args.name;
    let run_impl = optional_run_method(lifecycle);
    let shutdown_impl = optional_shutdown_method(lifecycle);

    quote! {
        #original_impl

        impl #impl_generics ::rust_supervisor::role::traits::supervisor::SupervisorRole
            for #self_ty
            #where_clause
        {
            fn build_tree<'a>(
                &'a mut self,
                ctx: &'a ::rust_supervisor::role::context::supervisor::SupervisorContext,
            ) -> impl ::std::future::Future<
                Output = ::rust_supervisor::role::result::supervisor::SupervisorResult<
                    ::rust_supervisor::spec::supervisor::SupervisorSpec,
                >,
            > + Send + 'a {
                async move {
                    self.build_tree(ctx).await
                }
            }

            #run_impl

            #shutdown_impl
        }

        impl #impl_generics #self_ty
            #where_clause
        {
            pub fn supervisor_adapter(
                self,
            ) -> ::rust_supervisor::role::adapter::supervisor::SupervisorRoleAdapter<Self> {
                ::rust_supervisor::role::adapter::supervisor::SupervisorRoleAdapter::new(self)
            }

            pub fn child_spec(
                self,
            ) -> ::std::result::Result<
                ::rust_supervisor::spec::child::ChildSpec,
                ::rust_supervisor::error::types::SupervisorError,
            > {
                let factory: ::std::sync::Arc<dyn ::rust_supervisor::task::factory::TaskFactory> =
                    ::std::sync::Arc::new(self.supervisor_adapter());
                ::rust_supervisor::spec::child_builder::ChildSpecBuilder::worker(
                    ::rust_supervisor::id::types::ChildId::new(#id),
                    #name,
                    ::rust_supervisor::spec::child::TaskKind::AsyncWorker,
                    factory,
                )
                .task_role(::rust_supervisor::policy::task_role_defaults::TaskRole::Supervisor)
                .criticality(::rust_supervisor::spec::child::Criticality::Critical)
                .build()
            }
        }
    }
}

/// Generates an optional `run` lifecycle bridge.
///
/// # Arguments
///
/// - `lifecycle`: Parsed supervisor lifecycle metadata for the annotated block.
///
/// # Returns
///
/// Returns tokens that override `SupervisorRole::run` only when the user
/// defined an inherent `run` method.
fn optional_run_method(lifecycle: &SupervisorLifecycleImpl) -> TokenStream {
    if !lifecycle.has_run {
        return TokenStream::new();
    }

    quote! {
        fn run<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::supervisor::SupervisorContext,
            handle: &'a ::rust_supervisor::control::handle::SupervisorHandle,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::supervisor::SupervisorResult<()>,
        > + Send + 'a {
            async move {
                self.run(ctx, handle).await
            }
        }
    }
}

/// Generates an optional `shutdown` lifecycle bridge.
///
/// # Arguments
///
/// - `lifecycle`: Parsed supervisor lifecycle metadata for the annotated block.
///
/// # Returns
///
/// Returns tokens that override `SupervisorRole::shutdown` only when the user
/// defined an inherent `shutdown` method.
fn optional_shutdown_method(lifecycle: &SupervisorLifecycleImpl) -> TokenStream {
    if !lifecycle.has_shutdown {
        return TokenStream::new();
    }

    quote! {
        fn shutdown<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::supervisor::SupervisorContext,
            handle: &'a ::rust_supervisor::control::handle::SupervisorHandle,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::supervisor::SupervisorResult<()>,
        > + Send + 'a {
            async move {
                self.shutdown(ctx, handle).await
            }
        }
    }
}
