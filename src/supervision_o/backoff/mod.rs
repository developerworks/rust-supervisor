//! 重连与重试场景下的指数退避实现。
//!
//! 本模块按结构体名称组织文件：
//! - [`ReconnectBackoff`]：退避状态结构；
//! - `reconnect_backoff_impl`：指数退避与等待逻辑实现。

mod reconnect_backoff;
mod reconnect_backoff_impl;

pub use reconnect_backoff::ReconnectBackoff;
