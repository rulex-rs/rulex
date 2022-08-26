//! # pomsky-syntax
//!
//! Crate for parsing [pomsky expressions](https://pomsky-lang.org).
//!
//! ## Usage
//!
//! ```
//! let (result, warnings) = pomsky_syntax::parse("let x = 'test'; x*", 256).unwrap();
//! assert!(warnings.is_empty());
//! ```

mod parse;
mod span;
mod util;

pub mod error;
pub mod exprs;
pub mod warning;

pub use parse::parse;
pub use span::Span;
pub use util::find_suggestion;