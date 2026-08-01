//! Command execution, grouped by domain. Each module adds `exec_*`
//! methods to `Engine`; the dispatcher lives in `engine.rs`.

mod assembly;
mod collision;
mod document_io;
mod feature;
mod part;
mod sketch;
