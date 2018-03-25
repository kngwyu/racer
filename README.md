# racer-nightly

[![Build Status](https://travis-ci.org/kngwyu/racer-nightly.svg?branch=master)](https://travis-ci.org/kngwyu/racer-nightly)

`rustc-ap-syntax` version of [racer](https://github.com/racer-rust/racer) for nightly Rust, based on https://github.com/racer-rust/racer/commit/86be1103e804bae2d0c324a94984abe3e12a3db5

## status
- [x] [#826](https://github.com/racer-rust/racer/issues/826)
- [x] [#785](https://github.com/racer-rust/racer/issues/785)
- [x] [#815](https://github.com/racer-rust/racer/issues/815)
- [x] replace `syntex_syntax` with `rustc-ap-syntax`
- [x] support [use_nested_groups](https://github.com/rust-lang/rust/issues/44494)
- [ ] rewrite file cache using `cargo metadata`
- [ ] get definition of macros in other crates
- [ ] complete `try_trait` support
- [ ] completion based on trait bound
