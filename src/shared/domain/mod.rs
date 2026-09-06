//! Shared, provider-agnostic domain building blocks used across bounded contexts.

/// Generates a validated, non-empty string newtype.
///
/// Each bounded context keeps its own domain error type; pass it as `$err` so
/// the generated constructor returns that context's error on blank input
/// instead of merging error types across contexts.
macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal, $err:ident) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates the value and rejects blank input.
            pub fn new(value: impl Into<String>) -> Result<Self, $err> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err($err::EmptyValue { field: $field });
                }
                Ok(Self(value))
            }

            /// Returns the original value supplied at the boundary.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

pub(crate) use text_value;
