//! A [`JobStep`] is the grouping of a [`Processor`], some identification
//! details such as a name and description, and the position within a series of
//! steps.
//!
//! It is similar to a [`Step`], except that it is tied to a [`Job`], instead
//! of a [`Task`]. The difference is that tasks are pre-defined job
//! templates that can be executed. Once a task is executed, it will spin
//! off a job, with its own steps, and run those steps.
//!
//! [`Processor`]: crate::Processor
//! [`Step`]: crate::resources::Step

use crate::resources::{Job, Step};
use crate::schema::job_steps;
use crate::Database;
use crate::Processor;
use automaat_core::Context;
use chrono::prelude::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use juniper::GraphQLEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{AsRef, TryFrom};
use std::error::Error;
use tera::{Context as TContext, Tera};

const INVALID_SERIALIZED_DATA: &str = "unexpected serialized data stored in database";

/// Contains all the data that can be used in processor templates.
#[derive(Serialize)]
struct TemplateData<'a> {
    /// The variable values defined by the task for which the values are
    /// provided when a job is created.
    var: HashMap<&'a str, &'a str>,

    /// System level variables that cannot be altered.
    sys: SystemVariables<'a>,
}

/// Contains all exposed system variables.
#[derive(Serialize)]
struct SystemVariables<'a> {
    /// Contains the output of the previous step that ran as part of this job.
    ///
    /// This value is an empty string if this is the first step to run, or the
    /// previous step provided no output.
    #[serde(rename = "previous step output")]
    step_output: &'a str,

    /// Contains the path to the current workspace.
    #[serde(rename = "workspace path")]
    workspace_path: &'a str,
}

/// The status of the job step.
#[derive(Clone, Copy, Debug, DbEnum, GraphQLEnum, Serialize, Deserialize)]
#[PgType = "JobStepStatus"]
#[graphql(name = "JobStepStatus")]
pub enum Status {
    /// The job step has been created, but is not yet ready to run.
    Initialized,

    /// The job step is waiting and ready to run.
    Pending,

    /// The job step is currently running and will either fail, or succeed.
    Running,

    /// The job step failed to run due to an unforeseen error.
    Failed,

    /// The job step was cancelled, and will not run anymore.
    Cancelled,

    /// The job step ran and succeeded.
    Ok,
}

/// The model representing a job step stored in the database.
#[derive(
    Clone, Debug, Deserialize, Serialize, AsChangeset, Associations, Identifiable, Queryable,
)]
#[belongs_to(Job)]
#[table_name = "job_steps"]
pub(crate) struct JobStep {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) processor: serde_json::Value,
    pub(crate) position: i32,
    pub(crate) started_at: Option<NaiveDateTime>,
    pub(crate) finished_at: Option<NaiveDateTime>,
    pub(crate) status: Status,
    pub(crate) output: Option<String>,
    pub(crate) job_id: i32,
}

impl JobStep {
    /// Returns the processor object attached to this job step.
    ///
    /// Given that jobs are historical entities, and processor object layouts
    /// can change between versions, this method returns an Option enum.
    ///
    /// If `None` is returned, it means the processor data could not be
    /// deserialized into the processor type.
    pub(crate) fn processor(&self) -> Option<Processor> {
        serde_json::from_value(self.processor.clone()).ok()
    }

    pub(crate) fn job(&self, conn: &Database) -> QueryResult<Job> {
        use crate::schema::jobs::dsl::*;

        jobs.filter(id.eq(self.job_id)).first(&**conn)
    }

    pub(crate) fn run(
        &mut self,
        conn: &Database,
        context: &Context,
        input: Option<&str>,
    ) -> Result<Option<String>, Box<dyn Error>> {
        self.start(conn)?;

        // TODO: this needs to go in a transaction, and the changes reverted if
        // they can't be saved... Also goes for many other places.

        let result = match self.formalize_processor(input, context, conn) {
            Ok(p) => p.run(context),
            Err(err) => Err(format!("job processor cannot be deserialized: {}", err).into()),
        };

        match result {
            Ok(output) => {
                self.finished(conn, Status::Ok, output.clone())?;
                Ok(output)
            }
            Err(err) => {
                self.finished(conn, Status::Failed, Some(err.to_string()))?;
                Err(err)
            }
        }
    }

    fn start(&mut self, conn: &Database) -> QueryResult<()> {
        self.status = Status::Running;
        self.started_at = Some(Utc::now().naive_utc());

        match self.save_changes::<Self>(&**conn) {
            Ok(_) => Ok(()),
            Err(err) => {
                self.status = Status::Failed;
                Err(err)
            }
        }
    }

    fn finished(
        &mut self,
        conn: &Database,
        status: Status,
        output: Option<String>,
    ) -> QueryResult<()> {
        self.finished_at = Some(Utc::now().naive_utc());
        self.status = status;
        self.output = output;

        self.save_changes::<Self>(&**conn).map(|_| ())
    }

    /// Takes the associated job step processor, and formalizes its definition
    /// by replacing any templated variables.
    fn formalize_processor(
        &mut self,
        input: Option<&str>,
        context: &Context,
        conn: &Database,
    ) -> Result<Processor, Box<dyn Error>> {
        let variables = self
            .job(conn)
            .and_then(|j| j.variables(conn))
            .map_err(Into::<Box<dyn Error>>::into)?;

        let var = variables
            .iter()
            .map(|v| (v.key.as_str(), v.value.as_str()))
            .collect();

        let sys = SystemVariables {
            step_output: input.unwrap_or(""),
            workspace_path: context.workspace_path().to_str().unwrap_or(""),
        };

        // Build a dataset of key/value pairs that can be used in the template
        // as variables and their substituted values.
        let data = TemplateData { var, sys };

        // The processor is serialized as `{ "ProcessorType": { ... } }` in the
        // database in order for Serde to know to which processor to deserialize
        // the JSON to.
        //
        // In this case, we want the configuration of the processor as a JSON
        // object, so we take "all values" (there's only one, the `{ ... }`
        // part).
        let mut processor = self.processor.clone();
        let config = processor
            .as_object_mut()
            .ok_or(INVALID_SERIALIZED_DATA)?
            .values_mut()
            .flat_map(serde_json::Value::as_object_mut)
            .next()
            .ok_or(INVALID_SERIALIZED_DATA)?;

        // process all values in the processor configuration as their own
        // templates.
        config
            .values_mut()
            .try_for_each(|v| self.formalize_value(v, &data))?;

        serde_json::from_value(processor).map_err(Into::into)
    }

    // Take a mutable JSON value reference, and a dataset of key/value pairs,
    // and formalize the final JSON value using a Jinja-like templating language
    // (using the Tera crate).
    //
    // If the JSON value is an array, the function recurses over the values
    // within that array.
    //
    // If the value is `null`, it is ignored.
    //
    // If the leaf JSON value is anything other than a string (or null), this
    // function returns an error.
    fn formalize_value(
        &self,
        value: &mut serde_json::Value,
        data: &TemplateData<'_>,
    ) -> Result<(), String> {
        if value.is_array() {
            return value
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .try_for_each(|v| self.formalize_value(v, data));
        };

        if value.is_null() {
            return Ok(());
        } else if !value.is_string() {
            return Err(INVALID_SERIALIZED_DATA.to_owned());
        };

        let context = TContext::from_serialize(data).map_err(|e| e.to_string())?;

        let mut tera = Tera::default();
        tera.add_raw_template("processor configuration", value.as_str().unwrap())
            .map_err(|e| e.to_string())?;

        match tera.render("processor configuration", context) {
            Ok(string) => *value = string.into(),
            Err(err) => {
                use tera::ErrorKind::*;

                let string = match err.kind {
                    FilterNotFound(string) => format!("missing template filter: {}", string),
                    TestNotFound(string) => format!("missing template test: {}", string),
                    FunctionNotFound(string) => format!("missing template function: {}", string),
                    Json(string) => format!("template json error: {}", string),
                    _ => match err.source() {
                        Some(source) => format!("template error: {}", source.to_string()),
                        None => format!("unknown template error: {}", err.to_string()),
                    },
                };

                return Err(string);
            }
        };

        Ok(())
    }
}

/// Contains all the details needed to store a job step in the database.
///
/// Use [`NewJobStep::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewJobStep<'a> {
    name: &'a str,
    description: Option<&'a str>,
    processor: Processor,
    position: i32,
    started_at: Option<NaiveDateTime>,
    finished_at: Option<NaiveDateTime>,
    output: Option<&'a str>,
    status: Status,
}

impl<'a> NewJobStep<'a> {
    /// Initialize a `NewJobStep` struct, which can be inserted into the
    /// database using the [`NewStep#add_to_job`] method.
    pub(crate) const fn new(
        name: &'a str,
        description: Option<&'a str>,
        processor: Processor,
        position: i32,
    ) -> Self {
        Self {
            name,
            description,
            processor,
            position,
            started_at: None,
            finished_at: None,
            output: None,
            status: Status::Initialized,
        }
    }

    /// Add a step to a [`Job`], by storing it in the database as an
    /// association.
    ///
    /// Requires a reference to a `Job`, in order to create the correct data
    /// reference.
    ///
    /// This method can return an error if the database insert failed, or if the
    /// associated processor is invalid.
    pub(crate) fn add_to_job(self, conn: &Database, job: &Job) -> Result<(), Box<dyn Error>> {
        use crate::schema::job_steps::dsl::*;

        self.processor.validate()?;

        let values = (
            name.eq(&self.name),
            description.eq(&self.description),
            processor.eq(serde_json::to_value(self.processor)?),
            position.eq(self.position),
            started_at.eq(self.started_at),
            finished_at.eq(self.finished_at),
            status.eq(Status::Pending),
            output.eq(&self.output),
            job_id.eq(job.id),
        );

        diesel::insert_into(job_steps)
            .values(values)
            .execute(&**conn)
            .map(|_| ())
            .map_err(Into::into)
    }
}

pub(crate) mod graphql {
    //! All GraphQL related functionality is encapsulated in this module. The
    //! relevant functions and structs are re-exported through
    //! [`crate::graphql`].
    //!
    //! API documentation in this module is also used in the GraphQL API itself
    //! as documentation for the clients.
    //!
    //! You can browse to `/graphql/playground` to see all relevant query,
    //! mutation, and type documentation.

    use super::*;
    use juniper::{object, FieldResult, ID};

    #[object(Context = Database)]
    impl JobStep {
        /// The unique identifier for a specific job step.
        fn id() -> ID {
            ID::new(self.id.to_string())
        }

        /// A descriptive name of the job step.
        fn name() -> &str {
            &self.name
        }

        /// An (optional) detailed description of the functionality provided by
        /// this job step.
        ///
        /// A description _might_ be markdown formatted, and should be parsed
        /// accordingly by the client.
        fn description() -> Option<&str> {
            self.description.as_ref().map(String::as_ref)
        }

        /// The processor used to run the job step.
        fn processor() -> Option<Processor> {
            self.processor()
        }

        /// The position of the step in a job, compared to other steps in the
        /// same job. A lower number means the step runs earlier in the job.
        fn position() -> i32 {
            self.position
        }

        fn started_at() -> Option<DateTime<Utc>> {
            self.started_at.map(|t| DateTime::from_utc(t, Utc))
        }

        fn finished_at() -> Option<DateTime<Utc>> {
            self.finished_at.map(|t| DateTime::from_utc(t, Utc))
        }

        fn status() -> Status {
            self.status
        }

        /// The output of the step, available in different formats.
        fn output() -> StepOutput<'_> {
            StepOutput(self.output.as_ref().map(String::as_ref))
        }

        /// The job to which the step belongs.
        ///
        /// This field can return `null`, but _only_ if a database error
        /// prevents the data from being retrieved.
        ///
        /// Every job step is _always_ attached to a job, so in normal
        /// circumstances, this field will always return the relevant job
        /// details.
        ///
        /// If a `null` value is returned, it is up to the client to decide the
        /// best course of action. The following actions are advised, sorted by
        /// preference:
        ///
        /// 1. continue execution if the information is not critical to success,
        /// 2. retry the request to try and get the relevant information,
        /// 3. disable parts of the application reliant on the information,
        /// 4. show a global error, and ask the user to retry.
        fn job(context: &Database) -> FieldResult<Option<Job>> {
            self.job(context).map(Some).map_err(Into::into)
        }
    }

    /// The output of the step, presented in different formats.
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub(crate) struct StepOutput<'a>(Option<&'a str>);

    #[object]
    impl<'a> StepOutput<'a> {
        /// The step output in text format.
        fn text() -> Option<&str> {
            self.0
        }

        /// The step output in HTML format.
        ///
        /// The HTML is generated from the text output, parsed as markdown.
        fn html() -> Option<String> {
            use pulldown_cmark::{html, Options, Parser};

            match self.0 {
                None => None,
                Some(output) => {
                    let mut options = Options::empty();
                    options.insert(Options::ENABLE_STRIKETHROUGH);
                    let parser = Parser::new_ext(output, options);
                    let mut html = String::new();
                    html::push_html(&mut html, parser);

                    Some(html)
                }
            }
        }
    }
}

impl<'a> TryFrom<&'a Step> for NewJobStep<'a> {
    type Error = serde_json::Error;

    fn try_from(step: &'a Step) -> Result<Self, Self::Error> {
        Ok(Self::new(
            &step.name,
            step.description.as_ref().map(String::as_ref),
            serde_json::from_value(step.processor.clone())?,
            step.position,
        ))
    }
}
