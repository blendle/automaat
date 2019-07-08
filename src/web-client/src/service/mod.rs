//! The list of services used in the application.

mod graphql;
mod shortcut;

pub(crate) use graphql::Service as GraphqlService;
pub(crate) use shortcut::Service as ShortcutService;
