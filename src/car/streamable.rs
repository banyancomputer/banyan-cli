use std::io::{Error, Read, Seek, Write};

/// Custom Stream-Based Serialization
pub trait Streamable: Sized {
    type StreamError;
    /// Read the bytes
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self, Self::StreamError>;
    /// Write the bytes
    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<(), Self::StreamError>;
}

/// Macro for generating a serialization test for any type which conforms to the Streamable trait
#[allow(unused_macros)]
macro_rules! streamable_tests {
    ($(
        <$type:ty, $error:ty>:
        $name:ident: $value:expr,
    )*) => {
    $(
        mod $name {
            #[allow(unused_imports)]
            use $crate::car::Streamable;
            #[allow(unused_imports)]
                        #[allow(unused_imports)]
            use std::io::{Read, Write, Cursor, SeekFrom, Seek};
            #[allow(unused_imports)]

            #[test]
            fn to_from_bytes() -> Result<(), $error> {
                // Serialize
                let mut bytes = Cursor::new(<Vec<u8>>::new());
                $value.write_bytes(&mut bytes)?;
                // Rewind
                bytes.seek(SeekFrom::Start(0))?;
                // Reconstruct
                let new_value = <$type>::read_bytes(&mut bytes)?;
                // Reserialize
                let mut new_bytes = Cursor::new(<Vec<u8>>::new());
                new_value.write_bytes(&mut new_bytes)?;
                // Assert equality of byte arrays
                assert_eq!(bytes.into_inner(), new_bytes.into_inner());
                // Ok
                Ok(())
            }
        }
    )*
    }
}

// Doing this allows us to use the macro within the crate
#[allow(unused_imports)]
pub(crate) use streamable_tests;
