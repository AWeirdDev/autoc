//! Autocompletion for CXX projects.

mod completions;
mod cxstring;
mod diagnostics;
mod doc;

// re-export
pub use libclang_sys as sys;

pub mod prelude {
    pub use crate::completions::*;
    pub use crate::cxstring::*;
    pub use crate::diagnostics::*;
    pub use crate::doc::*;
}
