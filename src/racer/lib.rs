#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", allow(clippy))]
#![cfg_attr(all(feature = "clippy", not(test)), deny(print_stdout))]

extern crate cargo;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
extern crate syntax;

#[macro_use]
mod testutils;

mod ast;
mod codecleaner;
mod codeiter;
mod core;
mod fileres;
mod matchers;
mod nameres;
mod scopes;
mod snippets;
mod typeinf;
mod util;

pub use core::{Coordinate, FileCache, FileLoader, Location, Point, Session, SourceByteRange};
pub use core::{Match, MatchType, PathSearch};
pub use core::{complete_from_file, complete_fully_qualified_name, find_definition, to_coords,
               to_point};
pub use snippets::snippet_for_match;
pub use util::expand_ident;

pub use util::{get_rust_src_path, RustSrcPathError};
