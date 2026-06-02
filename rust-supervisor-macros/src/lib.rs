//! Procedural macro entry points for rust-supervisor role contracts.
//!
//! This crate is intentionally small. Runtime types remain in the main
//! `rust-supervisor` crate while this crate owns compile-time code generation.

mod attribute;
mod expand;
mod parse;

use proc_macro::TokenStream;

/// Marks an inherent impl block as a service role contract.
///
/// # Arguments
///
/// - `args`: Attribute arguments that describe service metadata.
/// - `item`: Inherent impl block that contains lifecycle methods.
///
/// # Returns
///
/// Returns generated Rust code for the original impl block and the service
/// role bridge.
///
/// # Examples
///
/// ```ignore
/// use rust_supervisor_macros::service;
///
/// struct QuoteService;
///
/// #[service(id = "quote-service", name = "Quote Service")]
/// impl QuoteService {
///     async fn run(
///         &mut self,
///         ctx: &rust_supervisor::role::context::service::ServiceContext,
///     ) -> rust_supervisor::role::result::service::ServiceResult<()> {
///         ctx.wait_shutdown().await;
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn service(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::service::service(args, item)
}

/// Marks an inherent impl block as a worker role contract.
///
/// # Arguments
///
/// - `args`: Attribute arguments that describe worker metadata.
/// - `item`: Inherent impl block that contains lifecycle methods.
///
/// # Returns
///
/// Returns generated Rust code for the original impl block and the worker role
/// bridge.
#[proc_macro_attribute]
pub fn worker(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::worker::worker(args, item)
}

/// Marks an inherent impl block as a job role contract.
///
/// # Arguments
///
/// - `args`: Attribute arguments that describe job metadata.
/// - `item`: Inherent impl block that contains lifecycle methods.
///
/// # Returns
///
/// Returns generated Rust code for the original impl block and the job role
/// bridge.
#[proc_macro_attribute]
pub fn job(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::job::job(args, item)
}

/// Marks an inherent impl block as a sidecar role contract.
///
/// # Arguments
///
/// - `args`: Attribute arguments that describe sidecar metadata.
/// - `item`: Inherent impl block that contains lifecycle methods.
///
/// # Returns
///
/// Returns generated Rust code for the original impl block and the sidecar role
/// bridge.
#[proc_macro_attribute]
pub fn sidecar(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::sidecar::sidecar(args, item)
}

/// Marks an inherent impl block as a supervisor role contract.
///
/// # Arguments
///
/// - `args`: Attribute arguments that describe supervisor metadata.
/// - `item`: Inherent impl block that contains lifecycle methods.
///
/// # Returns
///
/// Returns generated Rust code for the original impl block and the supervisor
/// role bridge.
#[proc_macro_attribute]
pub fn supervisor_role(args: TokenStream, item: TokenStream) -> TokenStream {
    attribute::supervisor_role::supervisor_role(args, item)
}
