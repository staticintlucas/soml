# soml <sup><sub>(smol toml)<sub></sup> [![Test Status]][actions]&thinsp;[![Crate Version]][crates]&thinsp;[![Rust Version]][crates]

[test status]: https://img.shields.io/github/actions/workflow/status/staticintlucas/soml/ci.yml?branch=main&label=tests&style=flat-square
[crate version]: https://img.shields.io/crates/v/soml?style=flat-square
[rust version]: https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fstaticintlucas%2Fsoml%2Fmain%2FCargo.toml&query=%24.package%5B%22rust-version%22%5D&style=flat-square&label=rust

[actions]: https://github.com/staticintlucas/soml/actions?query=branch%3Amain
[crates]: https://crates.io/crates/soml

A lightweight [Serde]-compatible [TOML][toml-lang] parser written in Rust

## *but why?*

The semi-official [toml][toml-rs] crate has lots of amazing features,
but that makes it really heavy in terms of binary size and compile times.
For one of my projects toml accounted for over 15% (~260 kB) of the binary size
despite only being used to read one small config file.

This project designed to sit at the opposite end of the spectrum,
offering basic TOML support with a much smaller footprint.

We also try to maintain API-compatibility with the deserialisation portion of the toml
crate, so migrating to (or from) soml should be relatively easy.

[serde]: https://serde.rs/
[toml-lang]: https://toml.io/
[toml-rs]: https://github.com/toml-lang/toml

## Licence

Licensed under either of

* Apache License, Version 2.0 ([LICENCE-APACHE](LICENCE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0][apache-licence])
* MIT license ([LICENCE-MIT](LICENCE-MIT) or [http://opensource.org/licenses/MIT][mit-licence])

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.

[apache-licence]: http://www.apache.org/licenses/LICENSE-2.0
[mit-licence]: http://opensource.org/licenses/MIT
