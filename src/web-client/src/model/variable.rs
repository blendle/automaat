//! A variable belonging to a task.

use crate::graphql::fetch_task_details::{
    FetchTaskDetailsTaskVariables, FetchTaskDetailsTaskVariablesValueAdvertisers,
};
use crate::model::task::Id as TaskId;

/// The variable model.
#[derive(Clone, Debug)]
pub(crate) struct Variable<'a> {
    /// The inner representation of the task variable, as defined by the server.
    inner: &'a FetchTaskDetailsTaskVariables,
}

/// A collection of task details needed to expose the details of a value
/// advertisement in the task details overview.
#[derive(Debug)]
pub(crate) struct ValueAdvertiser<'a> {
    /// The ID of the task advertising a given variable value.
    pub(crate) task_id: TaskId,

    /// The name of the advertising task.
    pub(crate) name: &'a str,

    /// The description of the advertising task.
    pub(crate) description: Option<&'a str>,
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

    /// An optional example value set by the server for the variable.
    pub(crate) fn example_value(&self) -> Option<&str> {
        self.inner.example_value.as_ref().map(String::as_str)
    }

    /// An optional constraint on the set of values the variable can have.
    pub(crate) fn selection_constraint(&self) -> Option<Vec<&str>> {
        self.inner
            .constraints
            .selection
            .as_ref()
            .map(|v| v.iter().map(String::as_str).collect())
    }

    /// Return a list of task details that advertise their capability of
    /// providing a value for this variable.
    pub(crate) fn value_advertisers(&self) -> Vec<ValueAdvertiser<'a>> {
        self.inner
            .value_advertisers
            .iter()
            .map(Into::into)
            .collect()
    }
}

impl<'a> From<&'a FetchTaskDetailsTaskVariablesValueAdvertisers> for ValueAdvertiser<'a> {
    fn from(advert: &'a FetchTaskDetailsTaskVariablesValueAdvertisers) -> Self {
        Self {
            task_id: TaskId::new(advert.id.to_owned()),
            name: advert.name.as_str(),
            description: advert.description.as_ref().map(String::as_str),
        }
    }
}

impl<'a> From<&'a FetchTaskDetailsTaskVariables> for Variable<'a> {
    fn from(inner: &'a FetchTaskDetailsTaskVariables) -> Self {
        Self { inner }
    }
}
