//! The derived GraphQL query and mutation structures.

use graphql_client::GraphQLQuery;

/// Fetch the global application statistics from the server.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/fetch_statistics.graphql",
    response_derives = "Debug, Clone"
)]
pub(crate) struct FetchStatistics;

/// Search for a set of tasks at the server, based on a provided query.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/search_tasks.graphql",
    response_derives = "Debug, Clone"
)]
pub(crate) struct SearchTasks;

/// Fetch all relevant details of a single task.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/fetch_task_details.graphql",
    response_derives = "Debug, Clone"
)]
pub(crate) struct FetchTaskDetails;

/// Create a new job on the server, based on a task and the provided variables.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/create_job.graphql",
    response_derives = "Debug, Clone"
)]
pub(crate) struct CreateJob;

/// Fetch the details of a job.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/fetch_job_result.graphql",
    response_derives = "Debug, Clone"
)]
pub(crate) struct FetchJobResult;

/// Fetch the details of the active session (if any).
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/fetch_session_details.graphql",
    response_derives = "Debug, Clone"
)]
pub(crate) struct FetchSessionDetails;
