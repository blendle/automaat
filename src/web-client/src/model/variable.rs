//! A variable belonging to a task.

use crate::graphql::fetch_task_details::FetchTaskDetailsPipelineVariables;

/// The variable model.
#[derive(Clone, Debug)]
pub(crate) struct Variable<'a> {
    /// The inner representation of the task variable, as defined by the server.
    inner: &'a FetchTaskDetailsPipelineVariables,
}

impl<'a> Variable<'a> {
    /// The key of the variable.
    pub(crate) fn key(&self) -> &str {
        &self.inner.key
    }

    /// The description of the variable.
    ///
    /// The server does not require a variable to have a description, but for
    /// visualisation purposes it is fine to map an unset description to an
    /// empty string.
    pub(crate) fn description(&self) -> &str {
        self.inner.description.as_ref().map_or("", String::as_str)
    }

    /// An optional default value set by the server for the variable.
    pub(crate) fn default_value(&self) -> Option<&str> {
        self.inner.default_value.as_ref().map(String::as_str)
    }

    /// An optional constraint on the set of values the variable can have.
    pub(crate) fn selection_constraint(&self) -> Option<Vec<&str>> {
        self.inner
            .constraints
            .selection
            .as_ref()
            .map(|v| v.iter().map(String::as_str).collect())
    }
}

impl<'a> From<&'a FetchTaskDetailsPipelineVariables> for Variable<'a> {
    fn from(inner: &'a FetchTaskDetailsPipelineVariables) -> Self {
        Self { inner }
    }
}

// #[derive(Clone, Debug, Eq, PartialEq, Hash)]
// pub(crate) struct Id(String);
