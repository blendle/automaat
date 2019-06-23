use automaat_core::{Context, Processor as CoreProcessor};
use juniper::GraphQLInputObject;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::error;

// Macro to create all required processor implementations without having to
// change tens of lines for every new processor added.
//
// See the end of this file for the actual usage.
macro_rules! impl_processors {
    ($($name:ident: $processor:ident),+) => {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub(crate) enum Processor {
            $($processor($processor)),+
        }

        impl Processor {
            pub(crate) fn run(
                self,
                context: &Context,
            ) -> Result<Option<String>, Box<dyn error::Error>> {
                match self {
                    $(Processor::$processor(p) => p.run(context).map_err(Into::into)),+
                }
            }

            pub(crate) fn validate(&self) -> Result<(), Box<dyn error::Error>> {
                match self {
                    $(Processor::$processor(p) => p.validate().map_err(Into::into)),+
                }
            }
        }

        // Dynamically construct items by combining `$processor` and `Input` to
        // create types such as `GitClineInput`, etc...
        paste::item! {
            $(use [<processor_ $name _v1>]::{$processor, Input as [<$processor Input>]};)+

            // NOTE: GraphQL does not support union input types, so this struct
            // with one field for each (optional) processor type is the best we
            // can do for now without giving up the typed nature of the input
            // object.
            //
            // see: https://github.com/graphql/graphql-spec/issues/488
            #[derive(Clone, Debug, Serialize, Deserialize, GraphQLInputObject)]
            #[graphql(name = "ProcessorInput")]
            pub(crate) struct Input {
                $($name: Option<[<$processor Input>]>),+
            }
        }

        // Given a GraphQL processor input object `ProcessorInput`, check if it
        // contains exactly _one_ processor configuration, and convert from that
        // processor's input type into a regular processor type.
        //
        // For example, this is not allowed and will return an error:
        //
        // ```
        // ProcessorInput {
        //     git_clone: GitCloneInput { ... },
        //     shell_command: ShellCommandInput { ... },
        // }
        // ```
        //
        // But this is:
        //
        // ```
        // ProcessorInput {
        //     git_clone: GitCloneInput { ... },
        //     shell_command: null,
        // }
        // ```
        //
        // This is done to create a strongly-typed union-like input object when
        // creating pipelines using processor configurations.
        impl TryFrom<Input> for Processor {
            type Error = String;

            fn try_from(input: Input) -> Result<Self, Self::Error> {
                let mut i = 0;
                $(i = input.$name.iter().fold(i, |i, _| i + 1));+;

                if i != 1 {
                    return Err("must provide exactly one processor input value".into());
                }

                $(if let Some(processor) = input.$name {
                    return Ok(Processor::$processor(processor.into()));
                })+

                unreachable!()
            }
        }

        juniper::graphql_union!(Processor: () where Scalar = <S> |&self| {
            instance_resolvers: |_| {
                $(&$processor => match *self {
                    Processor::$processor(ref p) => Some(p),
                    _ => None
                }),+
            }
        });
    }
}

// This creates all "v1" processor types, and exposes them over GraphQL.
//
// Version 1 processors use "naked" names, meaning they are used as `GitClone`,
// not `GitCloneV1`. If we ever have a need to do breaking changes, we'll add a
// `GitClonev2` option alongside the regular `GitClone` one, and deprecate the
// regular one.
impl_processors! {
    git_clone:     GitClone,
    http_request:  HttpRequest,
    json_edit:     JsonEdit,
    print_output:  PrintOutput,
    redis_command: RedisCommand,
    shell_command: ShellCommand,
    sql_query:     SqlQuery,
    string_regex:  StringRegex
}
