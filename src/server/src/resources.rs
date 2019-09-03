mod global_variable;
mod job;
mod session;
mod step;
mod task;
pub(crate) mod variable;

pub(crate) use global_variable::graphql::GlobalVariableInput;
pub(crate) use job::step::{
    JobStep, NewJobStep, Status as JobStepStatus, StatusMapping as JobStepStatusMapping,
};
pub(crate) use job::variable::{graphql::JobVariableInput, JobVariable, NewJobVariable};
pub(crate) use job::{
    graphql::CreateJobFromTaskInput, Job, NewJob, StatusMapping as JobStatusMapping,
};
pub(crate) use session::graphql::{CreateSessionInput, UpdatePrivilegesInput};
pub(crate) use step::{graphql::CreateStepInput, NewStep, Step};
pub(crate) use task::{
    graphql::{CreateTaskInput, SearchTaskInput},
    NewTask, Task,
};
pub(crate) use variable::{graphql::CreateVariableInput, NewVariable, Variable};

/// Define what to do when a conflict occurs on object mutation.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, juniper::GraphQLEnum)]
pub(crate) enum OnConflict {
    Abort,
    Update,
}
