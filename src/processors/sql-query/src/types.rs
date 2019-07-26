//! SQL types.

use serde::{Deserialize, Serialize};

macro_rules! types {
    (
        $($type:ident {
            function_name: $func:expr,
            rust_input_field: $field:ident,
            rust_type: $inner:ty,
            graphql_input_field: $fname:expr,
            graphql_type: $name:expr,
        })+
    ) => {
        $(
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[cfg_attr(
            feature = "juniper",
            derive(juniper::GraphQLScalarValue),
            graphql(name = $name)
        )]
        #[allow(missing_copy_implementations, missing_docs)]
        pub struct $type($inner);
        )+

        /// GraphQL SQL Type.
        #[cfg_attr(
            feature = "juniper",
            derive(juniper::GraphQLObject),
            graphql(name = "SqlType"),
        )]
        #[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
        pub struct Type {
            $(
            #[cfg_attr(feature = "juniper", graphql(name = $fname))]
            #[allow(missing_docs)]
            $field: Option<$inner>,
            )+
        }

        paste::item! {
        impl Type {
            $(
            /// Create new SQL type.
            pub fn $func<T: Into<$inner>>(v: T) -> Self {
                Self { $field: Some(v.into()), ..Self::default() }
            }

            /// Check if value is of this type.
            pub fn [<is_ $func>](&self) -> bool {
                self.$field.is_some()
            }

            /// Get value of this type, if it matches the type.
            pub fn [<as_ $func>](&self) -> Option<&$inner> {
                self.$field.as_ref()
            }
            )+
        }
        }

        /// GraphQL SQL Type input.
        #[cfg(feature = "juniper")]
        #[graphql(name = "SqlTypeInput")]
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, juniper::GraphQLInputObject)]
        pub struct TypeInput {
            $(
            #[graphql(name = $fname)]
            $field: Option<$inner>,
            )+
        }

        #[cfg(feature = "juniper")]
        impl std::convert::TryFrom<TypeInput> for Type {
            type Error = ();

            fn try_from(input: TypeInput) -> Result<Self, ()> {
                $(
                if let Some(v) = input.$field {
                    return Ok(Self { $field: Some(v), ..Self::default() });
                }
                )+

                Err(())
            }
        }

    };
}

types! {
    Text {
        function_name: text,
        rust_input_field: text,
        rust_type: String,
        graphql_input_field: "text",
        graphql_type: "SqlText",
    }

    Int {
        function_name: int,
        rust_input_field: int,
        rust_type: i32,
        graphql_input_field: "int",
        graphql_type: "SqlInt",
    }

    Bool {
        function_name: bool,
        rust_input_field: bool_,
        rust_type: bool,
        graphql_input_field: "bool",
        graphql_type: "SqlBool",
    }
}
