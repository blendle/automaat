use crate::resources::{
    CreatePipelineInput, CreateTaskFromPipelineInput, NewPipeline, NewTask, Pipeline,
    SearchPipelineInput, Task, VariableValue,
};
use crate::Database;
use diesel::prelude::*;
use juniper::{object, Context, FieldError, FieldResult, RootNode, ID};
use std::convert::TryFrom;

impl Context for Database {}

pub(crate) type Schema = RootNode<'static, QueryRoot, MutationRoot>;
pub(crate) struct QueryRoot;
pub(crate) struct MutationRoot;

#[object(Context = Database)]
impl QueryRoot {
    /// Return a list of pipelines.
    ///
    /// You can optionally filter the returned set of pipelines by providing the
    /// `SearchPipelineInput` value.
    fn pipelines(
        context: &Database,
        search: Option<SearchPipelineInput>,
    ) -> FieldResult<Vec<Pipeline>> {
        use crate::schema::pipelines::dsl::*;
        let conn = &context.0;

        let mut query = pipelines.order(id).into_boxed();

        if let Some(search) = &search {
            if let Some(search_name) = &search.name {
                query = query.filter(name.ilike(format!("%{}%", search_name)));
            };

            if let Some(search_description) = &search.description {
                query = query.or_filter(description.ilike(format!("%{}%", search_description)));
            };
        };

        query.load(conn).map_err(Into::into)
    }

    /// Return a list of tasks.
    fn tasks(context: &Database) -> FieldResult<Vec<Task>> {
        use crate::schema::tasks::dsl::*;

        tasks.order(id).load(&**context).map_err(Into::into)
    }

    /// Return a single pipeline, based on the pipeline ID.
    ///
    /// This query can return `null` if no pipeline is found matching the
    /// provided ID.
    fn pipeline(context: &Database, id: ID) -> FieldResult<Option<Pipeline>> {
        use crate::schema::pipelines::dsl::{id as pid, pipelines};

        pipelines
            .filter(pid.eq(id.parse::<i32>()?))
            .first(&**context)
            .optional()
            .map_err(Into::into)
    }

    /// Return a single task, based on the task ID.
    ///
    /// This query can return `null` if no task is found matching the
    /// provided ID.
    fn task(context: &Database, id: ID) -> FieldResult<Option<Task>> {
        use crate::schema::tasks::dsl::{id as tid, tasks};

        tasks
            .filter(tid.eq(id.parse::<i32>()?))
            .first(&**context)
            .optional()
            .map_err(Into::into)
    }
}

#[object(Context = Database)]
impl MutationRoot {
    /// Create a new pipeline.
    fn createPipeline(context: &Database, pipeline: CreatePipelineInput) -> FieldResult<Pipeline> {
        NewPipeline::try_from(&pipeline)?
            .create(context)
            .map_err(Into::into)
    }

    /// Create a task from an existing pipeline ID.
    ///
    /// Once the task is created, it will be scheduled to run immediately.
    fn createTaskFromPipeline(
        context: &Database,
        task: CreateTaskFromPipelineInput,
    ) -> FieldResult<Task> {
        let pipeline: Pipeline = {
            use crate::schema::pipelines::dsl::*;

            pipelines
                .filter(id.eq(task.pipeline_id.parse::<i32>()?))
                .first(&**context)
        }?;

        let variable_values = task
            .variables
            .into_iter()
            .map(Into::into)
            .collect::<Vec<VariableValue>>();

        if let Some(variable) = pipeline.get_missing_variable(context, &variable_values)? {
            return Err(format!(r#"missing variable: "{}""#, variable.key).into());
        };

        let mut new_task = NewTask::create_from_pipeline(context, &pipeline, &variable_values)
            .map_err(Into::<FieldError>::into)?;

        // TODO: when we have scheduling, we probably want this to be optional,
        // so that a task isn't always scheduled instantly.
        new_task.enqueue(context).map_err(Into::into)
    }
}
