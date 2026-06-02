//! Service role code generation.
//!
//! The generated code targets explicit runtime contracts in `rust_supervisor`
//! and keeps user-authored lifecycle methods in their original inherent impl.

use crate::parse::lifecycle_impl::LifecycleImpl;
use crate::parse::role_args::RoleArgs;
use proc_macro2::TokenStream;
use quote::quote;

/// Expands a parsed service role into runtime bridge code.
///
/// # Arguments
///
/// - `args`: Parsed service metadata from the attribute arguments.
/// - `lifecycle`: Parsed inherent impl block with lifecycle method metadata.
///
/// # Returns
///
/// Returns generated Rust tokens that include the original impl block, a
/// `ServiceRole` implementation, and service helper methods.
pub(crate) fn expand_service(args: &RoleArgs, lifecycle: &LifecycleImpl) -> TokenStream {
    debug_assert!(lifecycle.has_run);

    let original_impl = &lifecycle.item_impl;
    let self_ty = lifecycle.self_ty.as_ref();
    let id = &args.id;
    let name = &args.name;
    let init_impl = optional_init_method(lifecycle);
    let shutdown_impl = optional_shutdown_method(lifecycle);

    quote! {
        #original_impl

        impl ::rust_supervisor::role::traits::service::ServiceRole for #self_ty {
            #init_impl

            fn run<'a>(
                &'a mut self,
                ctx: &'a ::rust_supervisor::role::context::service::ServiceContext,
            ) -> impl ::std::future::Future<
                Output = ::rust_supervisor::role::result::service::ServiceResult<()>,
            > + Send + 'a {
                async move {
                    self.run(ctx).await
                }
            }

            #shutdown_impl
        }

        impl #self_ty {
            pub fn service_adapter(
                self,
            ) -> ::rust_supervisor::role::adapter::service::ServiceRoleAdapter<Self> {
                ::rust_supervisor::role::adapter::service::ServiceRoleAdapter::new(self)
            }

            pub fn child_spec(
                self,
            ) -> ::std::result::Result<
                ::rust_supervisor::spec::child::ChildSpec,
                ::rust_supervisor::error::types::SupervisorError,
            > {
                let factory: ::std::sync::Arc<dyn ::rust_supervisor::task::factory::TaskFactory> =
                    ::std::sync::Arc::new(self.service_adapter());
                ::rust_supervisor::spec::child_builder::ChildSpecBuilder::service(
                    ::rust_supervisor::id::types::ChildId::new(#id),
                    #name,
                    ::rust_supervisor::spec::child::TaskKind::AsyncWorker,
                    factory,
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
/// Returns tokens that override `ServiceRole::init` only when the user defined
/// an inherent `init` method.
fn optional_init_method(lifecycle: &LifecycleImpl) -> TokenStream {
    if !lifecycle.has_init {
        return TokenStream::new();
    }
    quote! {
        fn init<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::service::ServiceContext,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::service::ServiceResult<()>,
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
/// Returns tokens that override `ServiceRole::shutdown` only when the user
/// defined an inherent `shutdown` method.
fn optional_shutdown_method(lifecycle: &LifecycleImpl) -> TokenStream {
    if !lifecycle.has_shutdown {
        return TokenStream::new();
    }
    quote! {
        fn shutdown<'a>(
            &'a mut self,
            ctx: &'a ::rust_supervisor::role::context::service::ServiceContext,
        ) -> impl ::std::future::Future<
            Output = ::rust_supervisor::role::result::service::ServiceResult<()>,
        > + Send + 'a {
            async move {
                self.shutdown(ctx).await
            }
        }
    }
}
