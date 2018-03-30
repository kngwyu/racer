#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", allow(clippy))]
#![cfg_attr(all(feature = "clippy", not(test)), deny(print_stdout))]

extern crate cargo;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
extern crate rustc_errors;
extern crate syntax;
extern crate syntax_pos;

#[macro_use]
mod testutils;

mod core;
mod scopes;
mod ast;
mod typeinf;
mod nameres;
mod util;
mod codeiter;
mod codecleaner;
mod matchers;
mod snippets;
mod fileres;

pub use core::{complete_from_file, complete_fully_qualified_name, find_definition, to_coords,
               to_point};
pub use snippets::snippet_for_match;
pub use core::{Match, MatchType, PathSearch};
pub use core::{Coordinate, FileCache, FileLoader, Location, Point, Session, SourceByteRange};
pub use util::expand_ident;

pub use util::{get_rust_src_path, RustSrcPathError};
