use crate::resources::Job;
use crate::schema::job_variables;
use crate::Database;
use crate::SERVER_SECRET;
use diesel::prelude::*;
use diesel::sql_types::{Bytea, Text};
use serde::{Deserialize, Serialize};
use std::{error::Error, str};

/// The model representing a job variable definition (_with_ an actual value)
/// stored in the database.
#[derive(Clone, Debug, Deserialize, Serialize, Associations, Identifiable, Queryable)]
#[belongs_to(Job)]
#[table_name = "job_variables"]
pub(crate) struct JobVariable {
    pub(crate) id: i32,
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) job_id: i32,
}

sql_function!(fn pgp_sym_encrypt(data: Text, secret: Text) -> Bytea);
sql_function!(fn pgp_sym_decrypt(data: Bytea, secret: Text) -> Text);

/// Contains all the details needed to store a job variable in the database.
///
/// Use [`NewJobVariable::new`] to initialize this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NewJobVariable<'a> {
    key: &'a str,
    value: &'a str,
}

impl<'a> NewJobVariable<'a> {
    /// Initialize a `NewJobVariable` struct, which can be inserted into the
    /// database using the [`NewJobVariable#add_to_job`] method.
    pub(crate) const fn new(key: &'a str, value: &'a str) -> Self {
        Self { key, value }
    }

    pub(crate) const fn key(&self) -> &str {
        self.key
    }

    /// Add a variable to a [`Job`], by storing it in the database as an
    /// association.
    ///
    /// Requires a reference to a `Job`, in order to create the correct data
    /// reference.
    ///
    /// This method can return an error if the database insert failed.
    pub(crate) fn add_to_job(self, conn: &Database, job: &Job) -> Result<(), Box<dyn Error>> {
        use crate::schema::job_variables::dsl::*;

        self.validate_selection_constraint(conn, job)?;

        let secret = SERVER_SECRET.as_str();
        let values = (
            key.eq(&self.key),
            value.eq(pgp_sym_encrypt(&self.value, secret)),
            job_id.eq(job.id),
        );

        diesel::insert_into(job_variables)
            .values(values)
            .execute(&**conn)
            .map(|_| ())
            .map_err(Into::into)
    }

    /// If the variable has a job, and that job has a task, check if the task
    /// has any selection constraint set, and if so, check that the variable
    /// value matches that constraint.
    fn validate_selection_constraint(
        &self,
        conn: &Database,
        job: &Job,
    ) -> Result<(), Box<dyn Error>> {
        let task = match job.task(conn)? {
            None => return Ok(()),
            Some(task) => task,
        };

        let variable = match task.variable_with_key(self.key, conn)? {
            None => return Ok(()),
            Some(variable) => variable,
        };

        let selection = match variable.selection_constraint {
            None => return Ok(()),
            Some(selection) => selection,
        };

        if selection.contains(&self.value.to_owned()) {
            return Ok(());
        }

        Err(format!(
            r#"variable "{}" must be one of: {}"#,
            self.key,
            selection.join(", ")
        )
        .into())
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
    use juniper::GraphQLInputObject;

    /// Contains the variable values to be used in the relevant job.
    #[derive(Clone, Debug, Deserialize, Serialize, GraphQLInputObject)]
    pub(crate) struct JobVariableInput {
        pub(crate) key: String,
        pub(crate) value: String,
    }
}

impl<'a> From<&'a graphql::JobVariableInput> for NewJobVariable<'a> {
    fn from(input: &'a graphql::JobVariableInput) -> Self {
        Self::new(&input.key, &input.value)
    }
}
