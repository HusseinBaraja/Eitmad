macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
            schemars::JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(uuid::Uuid);

        impl $name {
            #[must_use]
            pub const fn new(value: uuid::Uuid) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn value(self) -> uuid::Uuid {
                self.0
            }
        }

        impl From<uuid::Uuid> for $name {
            fn from(value: uuid::Uuid) -> Self {
                Self::new(value)
            }
        }
    };
}

macro_rules! open_id {
    ($name:ident, $kind:literal) => {
        #[derive(
            Clone,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            schemars::JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Parses and validates an open protocol identifier.
            ///
            /// # Errors
            ///
            /// Returns [`crate::transport::IdentifierError`] when the value is
            /// not a lowercase, bounded protocol identifier.
            pub fn parse(
                value: impl Into<String>,
            ) -> Result<Self, crate::transport::IdentifierError> {
                let value = value.into();
                crate::transport::validate_open_identifier(&value, $kind)?;
                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<Deserializer>(
                deserializer: Deserializer,
            ) -> Result<Self, Deserializer::Error>
            where
                Deserializer: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

macro_rules! tagged_contract {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $variant:ident($payload:ty) => $wire:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Clone,
            Debug,
            PartialEq,
            Eq,
            serde::Serialize,
            serde::Deserialize,
            schemars::JsonSchema,
        )]
        #[serde(tag = "kind", content = "payload")]
        $vis enum $name {
            $(
                #[serde(rename = $wire)]
                $variant($payload),
            )+
        }

        impl $name {
            pub const IDS: &'static [&'static str] = &[$($wire),+];

            #[must_use]
            pub const fn kind(&self) -> &'static str {
                match self {
                    $(Self::$variant(_) => $wire),+
                }
            }
        }
    };
}
