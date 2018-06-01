# racer-nightly

[![Build Status](https://travis-ci.org/kngwyu/racer-nightly.svg?branch=master)](https://travis-ci.org/kngwyu/racer-nightly)

`rustc-ap-syntax` version of [racer](https://github.com/racer-rust/racer) for nightly Rust, based on https://github.com/racer-rust/racer/commit/86be1103e804bae2d0c324a94984abe3e12a3db5

# Warning
Now I'm applying many changes in this branch to upstream and this branch is no longer maintainanced(but still exists for personal exprimants). So please use upstream branch.

## status
- [x] [#826](https://github.com/racer-rust/racer/issues/826)
- [x] [#785](https://github.com/racer-rust/racer/issues/785)
- [x] [#815](https://github.com/racer-rust/racer/issues/815)
- [x] replace `syntex_syntax` with `rustc-ap-syntax`
- [x] support [use_nested_groups](https://github.com/rust-lang/rust/issues/44494)
- [x] rewrite get_crate_file using `cargo` crate
  - [x] cache `src_path`s of outer crates
- [x] method completion for closure args
- [ ] completion based on trait bound
  - [x] for function args
- [ ] get definition of macros in other crates
- [ ] complete `try_trait` support
- [ ] more precise research flag(e.g. `extern crate` in outer crates is not a module)



