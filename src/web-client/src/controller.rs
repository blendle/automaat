//! The controller handles UI events, translates them into updates on the model,
//! and schedules re-renders.

use crate::model::{job, statistics, task, tasks};
use crate::router::Route;
use crate::service::GraphqlService;
use crate::App;
use dodrio::{RootRender, VdomWeak};
use futures::{future, prelude::*};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::UnwrapThrowExt;
use wasm_timer::{Delay, Instant};

/// The main application controller.
#[derive(Clone, Debug, Default)]
pub(crate) struct Controller;

impl tasks::Actions for Controller {
    fn search(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
        query: String,
    ) -> Box<dyn Future<Item = (), Error = ()> + 'static> {
        use crate::graphql::search_tasks::{SearchTaskInput, Variables};
        use crate::graphql::SearchTasks;

        let variables = Variables {
            search: Some(SearchTaskInput {
                name: Some(query.clone()),
                description: Some(query),
            }),
        };

        let app = root.unwrap_mut::<App>();
        let lock = app.cloned_tasks();

        // We need to take ownership of all the tasks and swap them later,
        // because our future will outlive the lifetime of this function.
        let mut tasks = match lock.try_borrow_mut() {
            Ok(tasks) => tasks.clone(),
            Err(_) => return Box::new(future::err(())),
        };

        let fut = app
            .client
            .request(SearchTasks, variables)
            .then(|response| {
                response
                    .ok()
                    .and_then(|r| r.data)
                    .map(|d| d.tasks)
                    .ok_or(())
            })
            .and_then(move |search_results| {
                // The search result IDs are used to set the active set of
                // filtered tasks. This is a subset of all retrieved tasks.
                //
                // This allows us to keep a cache of all tasks we've ever
                // fetched for the duration of the session, without having to
                // re-fetch the data after each search query removes old data.
                let search_ids = search_results
                    .clone()
                    .into_iter()
                    .map(|r| task::Id::new(r.id))
                    .collect::<Vec<_>>();

                // Keep any existing tasks that have more details than this
                // search result can provide us (this is the case if a task was
                // opened before, and more details were fetched).
                let new_tasks = search_results
                    .into_iter()
                    .zip(search_ids.iter())
                    .filter_map(|(r, id)| if tasks.contains(id) { None } else { Some(r) })
                    .collect::<Vec<_>>();

                for task in new_tasks {
                    tasks.add(task.into())
                }

                tasks.filter_tasks(search_ids);

                let _ = lock.replace(tasks);
                vdom.render().map_err(|_| ())
            });

        Box::new(fut)
    }
}

impl task::Actions for Controller {
    fn activate_task(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
        id: task::Id,
    ) -> Box<dyn Future<Item = (), Error = ()>> {
        use crate::graphql::fetch_task_details::Variables;
        use crate::graphql::FetchTaskDetails;

        let app = root.unwrap_mut::<App>();
        let lock = app.cloned_tasks();

        // short-circuit: if the task exists, and has all the required details,
        // activate it, schedule a render and return.
        if let Ok(mut tasks) = app.tasks_mut() {
            if let Some(task) = tasks.get(&id) {
                if task.variables().is_some() {
                    let _ = tasks.activate_task(id).unwrap_throw();
                    return Box::new(vdom.render().map_err(|_| ()));
                }
            }
        }

        // We need to take ownership of all the tasks and swap them later,
        // because our future will outlive the lifetime of this function.
        let mut tasks = match lock.try_borrow() {
            Ok(tasks) => tasks.clone(),
            Err(_) => return Box::new(future::err(())),
        };

        let variables = Variables { id: id.to_string() };

        let fut = app
            .client
            .request(FetchTaskDetails, variables)
            .then(|response| {
                response
                    .ok()
                    .and_then(|r| r.data)
                    .and_then(|d| d.task)
                    .map(Into::into)
                    .ok_or(())
            })
            .then(move |new_tasks: Result<Vec<_>, _>| {
                tasks.append(new_tasks.unwrap_throw());
                let _ = tasks.activate_task(id);
                let _ = lock.replace(tasks);
                vdom.render().map_err(|_| ())
            });

        Box::new(fut)
    }

    fn run(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
        id: task::Id,
        variables: HashMap<String, String>,
    ) -> Box<dyn Future<Item = job::RemoteId, Error = ()>> {
        use crate::graphql::{create_job::*, CreateJob};

        let app = root.unwrap_mut::<App>();
        let tasks = app.tasks().unwrap_throw();
        let active_task = tasks.get(&id).unwrap_throw();

        // Prevent the creation of a new job if the active job is still running.
        //
        // This is also handled in the UI by disabling the button, but this is
        // the "one true check" that also works when trying to run a task
        // using keyboard shortcuts.
        if active_task.active_job().map_or(false, job::Job::is_running) {
            return Box::new(future::err(()));
        }

        let mut job = job::Job::default();
        job.variable_values = variables.clone();

        let input = CreateJobFromTaskInput {
            task_id: id.to_string(),
            variables: variables
                .into_iter()
                .filter_map(|(key, value)| {
                    if value.is_empty() {
                        return None;
                    };

                    Some(JobVariableInput { key, value })
                })
                .collect(),
        };

        let lock = app.cloned_tasks();
        let fut = app
            .client
            .request(CreateJob, Variables { job: input })
            .map_err(|err| vec![err.to_string()])
            .and_then(|response| {
                if let Some(err) = response.errors {
                    future::err(err.iter().map(|e| e.message.to_owned()).collect())
                } else if let Some(data) = response.data {
                    future::ok(job::RemoteId::new(data.create_job_from_task.id))
                } else {
                    future::err(vec![])
                }
            })
            .then(move |result| {
                let mut tasks = lock.try_borrow_mut().unwrap_throw();
                let task = tasks.get_mut(&id).unwrap_throw();

                match &result {
                    Ok(job_id) => job.remote_id = Some(job::RemoteId::new(job_id.to_string())),
                    Err(err) => job.status = job::Status::Failed(err.join("\n")),
                };

                task.activate_job(job);
                vdom.schedule_render();
                result.map_err(|_| ())
            });

        Box::new(fut)
    }

    fn reactivate_last_job(root: &mut dyn RootRender, vdom: VdomWeak, id: task::Id) {
        let app = root.unwrap_mut::<App>();
        let mut tasks = app.tasks_mut().unwrap_throw();
        let task = tasks.get_mut(&id).unwrap_throw();

        task.activate_last_job();
        vdom.schedule_render();
    }

    fn close_active_task(root: &mut dyn RootRender, vdom: VdomWeak) {
        let app = root.unwrap_mut::<App>();
        let mut tasks = app.tasks_mut().unwrap_throw();
        let active_task = tasks.active_task_mut().unwrap_throw();

        // It's currently not possible to close the active task if it still has
        // an actively running job.
        //
        // This is also handled in the UI by disabling the button, but this is
        // the "one true check" that also works when trying to close a task
        // using keyboard shortcuts.
        //
        // It _is_ possible to use the browser's back button, but there's
        // nothing we can do about that, and so far, there hasn't been an issue
        // with things breaking when doing so.
        if active_task.active_job().map_or(false, job::Job::is_running) {
            return;
        }

        tasks.disable_active_task();
        match tasks.active_task() {
            Some(task) => Route::Task(task.id()).set_path(),
            None => Route::Home.set_path(),
        }

        vdom.schedule_render();
    }
}

impl job::Actions for Controller {
    #[allow(clippy::wildcard_enum_match_arm)]
    fn poll_result(
        lock: Rc<RefCell<tasks::Tasks>>,
        vdom: VdomWeak,
        id: job::RemoteId,
        task_id: task::Id,
        client: GraphqlService,
    ) -> Box<dyn Future<Item = (), Error = ()> + 'static> {
        use crate::graphql::{fetch_job_result::*, FetchJobResult};
        use futures::future::{loop_fn, Loop};
        use graphql_client::Response;

        let tries = 0;
        let future = loop_fn(
            (tries, client, lock, id, task_id, vdom),
            |(tries, client, lock, id, task_id, vdom)| {
                let variables = Variables { id: id.to_string() };

                // After the first request to check if the job finished, each
                // subsequent request will be done after a small delay, to
                // prevent flooding the server with requests.
                let delay = move |response| {
                    let timeout = if tries == 0 { 0 } else { 500 };

                    Delay::new(Instant::now() + Duration::from_millis(timeout))
                        .map(|_| response)
                        .map_err(|_| vec![])
                };

                // Check the response of the server and either return any
                // errors returned by the server, or pass along the request
                // body.
                let handle_response = |response: Response<ResponseData>| {
                    if let Some(err) = response.errors {
                        Err(err.iter().map(|e| e.message.to_owned()).collect())
                    } else if let Some(data) = response.data {
                        match data.job {
                            None => Err(vec!["no job data returned".to_owned()]),
                            Some(job) => Ok(job),
                        }
                    } else {
                        Err(vec!["unknown server error".to_owned()])
                    }
                };

                // Update the job status, including the possible error or
                // success message, based on the server response.
                let update_state = move |result: Result<FetchJobResultJob, Vec<String>>| {
                    use job::Status;
                    use JobStatus::*;
                    use JobStepStatus as S;

                    let mut tasks = lock.try_borrow_mut().unwrap_throw();
                    let task = tasks.get_mut(&task_id).unwrap_throw();
                    let job = task
                        .jobs
                        .iter_mut()
                        .find(|j| j.remote_id.as_ref() == Some(&id))
                        .unwrap_throw();

                    job.status = match result {
                        Err(err) => Status::Failed(err.join("\n")),
                        Ok(result) => match result.status {
                            SCHEDULED | PENDING | RUNNING => Status::Delivered,
                            FAILED | CANCELLED | OK => match result.steps.as_ref() {
                                None => Status::Succeeded("task has no steps".to_owned()),
                                Some(s) => {
                                    let step = match s
                                        .iter()
                                        .find(|s| s.status == JobStepStatus::FAILED)
                                    {
                                        Some(s) => s,
                                        None => s.last().unwrap_throw(),
                                    };

                                    let message = step
                                        .output
                                        .html
                                        .as_ref()
                                        .map_or("unknown error".to_owned(), String::to_owned);

                                    match &step.status {
                                        S::OK => Status::Succeeded(message),
                                        S::INITIALIZED
                                        | S::PENDING
                                        | S::RUNNING
                                        | S::FAILED
                                        | S::CANCELLED => Status::Failed(message),
                                        _non_exhaustive => unreachable!(),
                                    }
                                }
                            },
                            _non_exhaustive => unreachable!(),
                        },
                    };

                    if tries > 120 && job.is_running() {
                        job.status =
                            Status::Failed("timeout waiting for job to complete".to_owned());
                    }

                    let status = job.status.clone();
                    drop(tasks);

                    Ok((lock, id, task_id, status))
                };

                // Depending on the new job status, either keep polling the
                // server for the final status, or break out of the loop.
                let new_client = client.clone();
                let retry_or_break = move |(lock, id, task_id, status)| {
                    vdom.schedule_render();

                    match status {
                        job::Status::Delivered => Ok(Loop::Continue((
                            tries + 1,
                            new_client,
                            lock,
                            id,
                            task_id,
                            vdom,
                        ))),
                        job::Status::Created => unreachable!(),
                        _ => Ok(Loop::Break(())),
                    }
                };

                client
                    .request(FetchJobResult, variables)
                    .map_err(|err| vec![err.to_string()])
                    .and_then(delay)
                    .and_then(handle_response)
                    .then(update_state)
                    .and_then(retry_or_break)
            },
        );

        Box::new(future)
    }

    fn abort(_root: &mut dyn RootRender, _vdom: VdomWeak, _id: job::RemoteId) {}
}

impl statistics::Actions for Controller {
    fn update_statistics(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
    ) -> Box<dyn Future<Item = (), Error = ()> + 'static> {
        use crate::graphql::fetch_statistics::*;
        use crate::graphql::FetchStatistics;

        let app = root.unwrap_mut::<App>();
        let stats = app.cloned_statistics();
        let fut = app
            .client
            .request(FetchStatistics, Variables)
            .then(|response| {
                response
                    .ok()
                    .and_then(|r| r.data)
                    .map(|d| (d.tasks, d.jobs))
                    .ok_or(())
            })
            .and_then(move |(tasks, jobs)| {
                let mut stats = stats.try_borrow_mut().unwrap_throw();

                let running = jobs
                    .iter()
                    .filter(|j| j.status == JobStatus::RUNNING)
                    .count();

                let failed = jobs
                    .iter()
                    .filter(|j| j.status == JobStatus::FAILED)
                    .count();

                stats.update(tasks.len(), running, failed);
                vdom.render().map_err(|_| ())
            });

        Box::new(fut)
    }
}
