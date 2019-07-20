mod global_variable;
mod job;
mod step;
mod task;
pub(crate) mod variable;

pub(crate) use global_variable::graphql::{GlobalVariableInput, OnConflict};
pub(crate) use job::step::{
    JobStep, NewJobStep, Status as JobStepStatus, StatusMapping as JobStepStatusMapping,
};
pub(crate) use job::variable::{graphql::JobVariableInput, JobVariable, NewJobVariable};
pub(crate) use job::{
    graphql::CreateJobFromTaskInput, Job, NewJob, StatusMapping as JobStatusMapping,
};
pub(crate) use step::{graphql::CreateStepInput, NewStep, Step};
pub(crate) use task::{
    graphql::{CreateTaskInput, SearchTaskInput},
    NewTask, Task,
};
pub(crate) use variable::{graphql::CreateVariableInput, NewVariable, Variable};
