use crate::resources::{
    variable, CreateJobFromPipelineInput, CreatePipelineInput, Job, NewJob, NewPipeline, Pipeline,
    SearchPipelineInput, VariableValue,
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

    /// Return a list of jobs.
    fn jobs(context: &Database) -> FieldResult<Vec<Job>> {
        use crate::schema::jobs::dsl::*;

        jobs.order(id).load(&**context).map_err(Into::into)
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

    /// Return a single job, based on the job ID.
    ///
    /// This query can return `null` if no job is found matching the
    /// provided ID.
    fn job(context: &Database, id: ID) -> FieldResult<Option<Job>> {
        use crate::schema::jobs::dsl::{id as tid, jobs};

        jobs.filter(tid.eq(id.parse::<i32>()?))
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

    /// Create a job from an existing pipeline ID.
    ///
    /// Once the job is created, it will be scheduled to run immediately.
    fn createJobFromPipeline(
        context: &Database,
        job: CreateJobFromPipelineInput,
    ) -> FieldResult<Job> {
        let pipeline: Pipeline = {
            use crate::schema::pipelines::dsl::*;

            pipelines
                .filter(id.eq(job.pipeline_id.parse::<i32>()?))
                .first(&**context)
        }?;

        let variable_values = job
            .variables
            .into_iter()
            .map(Into::into)
            .collect::<Vec<VariableValue>>();

        let pipeline_variables = pipeline.variables(context)?;

        if let Some(variables) = variable::missing_values(&pipeline_variables, &variable_values) {
            let keys = variables.iter().map(|v| v.key.as_str()).collect::<Vec<_>>();

            return Err(format!(r#"missing variable values: {}"#, keys.join(", ")).into());
        }

        if let Some(results) =
            variable::selection_constraint_mismatch(&pipeline_variables, &variable_values)
        {
            let variable = results[0].0;
            let value = results[0].1;

            // TODO: turn this into a structured error object, so we can expose
            // all the invalid variables at once.
            return Err(format!(
                r#"invalid variable value: "{}", must be one of: {:?}"#,
                value.key,
                variable
                    .selection_constraint
                    .as_ref()
                    .unwrap_or(&vec![])
                    .join(", ")
            )
            .into());
        }

        let mut new_job = NewJob::create_from_pipeline(context, &pipeline, &variable_values)
            .map_err(Into::<FieldError>::into)?;

        // TODO: when we have scheduling, we probably want this to be optional,
        // so that a job isn't always scheduled instantly.
        new_job.enqueue(context).map_err(Into::into)
    }
}
