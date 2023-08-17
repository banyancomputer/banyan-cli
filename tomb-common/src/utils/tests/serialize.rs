/// Macro for generating a serialization test for any type which conforms to the Serialize and Deserialize trait
#[macro_export]
macro_rules! serialization_tests {
    ($(
        $type:ty:
        $name:ident: $value:expr,
    )*) => {
    $(
        mod $name {
            use wnfs::common::dagcbor;
            use anyhow::Result;
            use super::*;
            #[test]
            fn dagcbor() -> Result<()> {
                // Serialize
                let mut bytes = dagcbor::encode($value)?;
                // Reconstruct
                let new_value = dagcbor::decode::<$type>(&bytes)?;
                // Reserialize
                let mut new_bytes = dagcbor::encode(&new_value)?;
                // Assert equality of byte arrays
                assert_eq!(bytes, new_bytes);
                // Ok
                Ok(())
            }
        }
    )*
    }
}