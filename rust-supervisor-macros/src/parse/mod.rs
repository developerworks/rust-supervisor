//! Parser modules for role contract macros.
//!
//! Parsers keep syntax validation separate from code generation, so macro error
//! messages can point at the original input tokens.

pub(crate) mod job_lifecycle_impl;
pub(crate) mod lifecycle_impl;
pub(crate) mod role_args;
pub(crate) mod sidecar_args;
pub(crate) mod sidecar_lifecycle_impl;
pub(crate) mod supervisor_lifecycle_impl;
pub(crate) mod worker_lifecycle_impl;
