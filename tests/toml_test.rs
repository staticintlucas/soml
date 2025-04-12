#![allow(missing_docs)]

mod utils;

mod toml_test {
    mod de {
        mod valid {
            include!(env!("TOML_TEST_DE_VALID"));
        }

        mod invalid {
            include!(env!("TOML_TEST_DE_INVALID"));
        }
    }

    mod ser {
        include!(env!("TOML_TEST_SER"));
    }
}
