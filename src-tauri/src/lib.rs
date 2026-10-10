pub mod asset;
pub mod embedding;
pub mod inference;
pub mod media;
pub mod models;
pub mod operation_jobs;
pub mod pdf;
pub mod process_guard;
pub mod runtime;
pub mod scratch;
pub mod settings;
pub mod store;

#[cfg(feature = "desktop")]
mod desktop;

#[cfg(feature = "desktop")]
pub use desktop::run;
