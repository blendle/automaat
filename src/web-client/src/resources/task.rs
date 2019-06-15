use self::fetch_task_details::FetchTaskDetailsTask;
use self::fetch_task_statuses::FetchTaskStatusesTasks;
use crate::views::PipelineDetailsView;
use chrono::prelude::*;
use core::ops::Deref;
use futures::future::Future;
use futures::future::{loop_fn, Either, Loop};
use graphql_client::web::Client;
use graphql_client::GraphQLQuery;
use std::collections::HashMap;
use std::time::Duration;
use url_serde::SerdeUrl as Url;
use wasm_timer::{Delay, Instant};

pub(crate) use self::fetch_task_details::{TaskStatus, TaskStepStatus};

type DateTimeUtc = DateTime<Utc>;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/task_statuses.graphql",
    response_derives = "Debug, Serialize, Deserialize"
)]
pub(crate) struct FetchTaskStatuses;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/create_task_from_pipeline.graphql"
)]
pub(crate) struct CreateTaskFromPipeline;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/fetch_task_details.graphql"
)]
pub(crate) struct FetchTaskDetails;

pub(crate) struct TaskStatuses(Vec<FetchTaskStatusesTasks>);

impl Deref for TaskStatuses {
    type Target = Vec<FetchTaskStatusesTasks>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) struct Task(FetchTaskDetailsTask);

impl Deref for Task {
    type Target = FetchTaskDetailsTask;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TaskStatuses {
    pub(crate) fn fetch() -> impl Future<Item = Self, Error = ()> {
        use self::fetch_task_statuses::Variables;

        let client = graphql_client::web::Client::new("/graphql");
        client
            .call(FetchTaskStatuses, Variables)
            .then(|result| match result {
                Ok(response) => match response.data {
                    Some(data) => Ok(Self(data.tasks)),
                    None => Err(()),
                },
                Err(_) => Err(()),
            })
    }
}

impl FetchTaskStatusesTasks {
    pub(crate) fn is_running(&self) -> bool {
        self.status == self::fetch_task_statuses::TaskStatus::RUNNING
    }

    pub(crate) fn is_failed(&self) -> bool {
        self.status == self::fetch_task_statuses::TaskStatus::FAILED
    }
}

impl CreateTaskFromPipeline {
    pub(crate) fn post(
        pipeline_id: String,
        variables: HashMap<String, String>,
    ) -> impl Future<Item = (), Error = ()> {
        use create_task_from_pipeline::*;

        let variables = variables
            .into_iter()
            .filter_map(|(key, value)| {
                if value.is_empty() {
                    return None;
                };

                Some(VariableValueInput { key, value })
            })
            .collect();

        let vars = Variables {
            task: CreateTaskFromPipelineInput {
                pipeline_id,
                variables,
            },
        };

        PipelineDetailsView::loading(true);

        graphql_client::web::Client::new("/graphql")
            .call(CreateTaskFromPipeline, vars)
            .map_err(|error| {
                Self::handle_failure(vec![error.to_string()]);
            })
            .and_then(|response| {
                if let Some(e) = response.errors {
                    Self::handle_failure(e.iter().map(|e| e.message.to_owned()).collect());
                    Box::new(futures::future::ok(()))
                } else if let Some(data) = response.data {
                    Self::handle_success(data.create_task_from_pipeline.id)
                } else {
                    Self::handle_failure(vec!["no data received from server".to_owned()]);
                    Box::new(futures::future::ok(()))
                }
            })
    }

    fn handle_success(task_id: String) -> Box<dyn Future<Item = (), Error = ()>> {
        use fetch_task_details::*;

        let client = Client::new("/graphql");
        let future = loop_fn(client, move |client| {
            client
                .call(
                    FetchTaskDetails,
                    Variables {
                        id: task_id.clone(),
                    },
                )
                .then(|result| match result {
                    Ok(response) => match response.errors {
                        Some(errors) => Err(errors.iter().map(|e| e.message.to_owned()).collect()),
                        None => match response.data {
                            Some(data) => match data.task {
                                Some(task) => Ok(task),
                                None => Err(vec!["no task details found".to_owned()]),
                            },
                            None => Err(vec!["invalid server response".to_owned()]),
                        },
                    },
                    Err(err) => Err(vec![err.to_string()]),
                })
                .map_err(PipelineDetailsView::add_errors)
                .and_then(|task| match task.status {
                    TaskStatus::PENDING | TaskStatus::RUNNING => Either::A(
                        Delay::new(Instant::now() + Duration::from_millis(500))
                            .map(|()| Loop::Continue(client))
                            .map_err(|_| ()),
                    ),
                    _ => {
                        PipelineDetailsView::add_task_status(&Task(task));
                        Either::B(futures::future::ok(Loop::Break(())))
                    }
                })
        });

        Box::new(future)
    }

    fn handle_failure(errors: Vec<String>) {
        PipelineDetailsView::add_errors(errors);
    }
}
