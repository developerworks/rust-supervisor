//! Sidecar role code generation.
//!
//! The generated code targets explicit runtime contracts in `rust_supervisor`
//! and keeps user-authored lifecycle methods in their original inherent impl.

use crate::parse::sidecar_args::SidecarArgs;
use crate::parse::sidecar_lifecycle_impl::SidecarLifecycleImpl;
use proc_macro2::TokenStream;
use quote::quote;

/// Expands a parsed sidecar role into runtime bridge code.
///
/// # Arguments
///
/// - `args`: Parsed sidecar metadata from the attribute arguments.
/// - `lifecycle`: Parsed inherent impl block with lifecycle method metadata.
///
/// # Returns
///
/// Returns generated Rust tokens that include the original impl block, a
/// `SidecarRole` implementation, and sidecar helper methods.
pub(crate) fn expand_sidecar(args: &SidecarArgs, lifecycle: &SidecarLifecycleImpl) -> TokenStream {
    debug_assert!(lifecycle.has_run);

    let original_impl = &lifecycle.item_impl;
    let self_ty = lifecycle.self_ty.as_ref();
    let (impl_generics, _, where_clause) = original_impl.generics.split_for_impl();
    let id = &args.id;
    let name = &args.name;
    let primary = &args.primary;
    let init_impl = optional_init_method(lifecycle);
    let shutdown_impl = optional_shutdown_method(lifecycle);

    quote! {
        #original_impl

        impl #impl_generics ::rust_supervisor::role::traits::sidecar::SidecarRole for #self_ty #where_clause {
            #init_impl

            fn run<'a>(
                &'a mut self,
                ctx: &'a ::rust_supervisor::role::context::sidecar::SidecarContext,
            ) -> impl ::std::future::Future<
                Output = ::rust_supervisor::role::result::sidecar::SidecarResult<()>,
            > + Send + 'a {
                async move {
                    self.run(ctx).await
                }
            }

            #shutdown_impl
        }

        impl #impl_generics #self_ty #where_clause {
            pub fn sidecar_adapter(
                self,
            ) -> ::rust_supervisor::role::adapter::sidecar::SidecarRoleAdapter<Self> {
                ::rust_supervisor::role::adapter::sidecar::SidecarRoleAdapter::new(
                    self,
                    ::rust_supervisor::id::types::ChildId::new(#primary),
                )
            }

            pub fn child_spec(
                self,
            ) -> ::std::result::Result<
                ::rust_supervisor::spec::child::ChildSpec,
                ::rust_supervisor::error::types::SupervisorError,
            > {
                ::rust_supervisor::spec::child_builder::ChildSpecBuilder::sidecar(
                    ::rust_supervisor::id::types::ChildId::new(#id),
                    #name,
                    ::rust_supervisor::spec::child::TaskKind::AsyncWorker,
                    ::std::sync::Arc::new(self.sidecar_adapter()),
                    ::rust_supervisor::policy::task_role_defaults::SidecarConfig::new(
                        ::rust_supervisor::id::types::ChildId::new(#primary),
                        true,
                    ),
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
/// Returns tokens that override `SidecarRole::init` only when the user defined
/// an inherent `init` method.
fn optional_init_method(lifecycle: &SidecarLifecycleImpl) -> TokenStream {
    if !lifecycle.has_init {
        return TokenStream::new();
    }
    quote! {
        fn init<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::sidecar::SidecarContext,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::sidecar::SidecarResult<()>,
        > + Send + 'a {
            async move {
                self.init(ctx).await
            }
        }
    }
}

/// Generates an optional `shutdown` lifecycle bridge.
///
/// # Arguments
///
/// - `lifecycle`: Parsed lifecycle metadata for the annotated impl block.
///
/// # Returns
///
/// Returns tokens that override `SidecarRole::shutdown` only when the user
/// defined an inherent `shutdown` method.
fn optional_shutdown_method(lifecycle: &SidecarLifecycleImpl) -> TokenStream {
    if !lifecycle.has_shutdown {
        return TokenStream::new();
    }
    quote! {
        fn shutdown<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::sidecar::SidecarContext,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::sidecar::SidecarResult<()>,
        > + Send + 'a {
            async move {
                self.shutdown(ctx).await
            }
        }
    }
}
