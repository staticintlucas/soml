// Tests don't use all our deps
#![allow(unused_crate_dependencies)]

use utils::EncodedItem;

mod utils;

include!(env!("TOML_TEST_TESTS"));
