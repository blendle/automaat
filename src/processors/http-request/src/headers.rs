use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "juniper", derive(juniper::GraphQLInputObject))]
#[cfg_attr(feature = "juniper", graphql(name = "RequestHeader"))]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct RequestHeader {
    pub name: HeaderName,
    pub value: String,
}

macro_rules! headers {
    ($(($const:ident, $name:expr);)+) => {
        /// The processor configuration.
        #[cfg_attr(feature = "juniper", derive(juniper::GraphQLEnum))]
        #[cfg_attr(feature = "juniper", graphql(name = "RequestHeaderName"))]
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        pub enum HeaderName {
            $($const,)+
        }

        impl From<HeaderName> for reqwest::header::HeaderName {
            fn from(method: HeaderName) -> Self {
                match method {
                    $(HeaderName::$const => Self::from_static($name),)+
                }
            }
        }
    };
}

headers! {
    (ACCEPT, "accept");
    (AUTHORIZATION, "authorization");
    (CONTENT_TYPE, "content-type");
    (WWW_AUTHENTICATE, "www-authenticate");
}
