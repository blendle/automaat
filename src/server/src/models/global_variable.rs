use crate::schema::global_variables;
use crate::SERVER_SECRET;
use diesel::prelude::*;
use diesel::sql_types::{Bytea, Text};

/// The model representing a global variable stored in the database.
#[derive(Debug, Identifiable, Queryable)]
pub(crate) struct GlobalVariable {
    pub(crate) id: i32,
    pub(crate) key: String,
    pub(crate) value: String,
}

impl GlobalVariable {
    /// Add a key filter to the global variable query.
    pub(crate) fn with_key(key: &str) -> WithKey<'_> {
        global_variables::key.eq(key)
    }

    /// Build a query that searches for a specific global variable, based on the
    /// provided key.
    #[allow(dead_code)]
    pub(crate) fn by_key(key: &str) -> ByKey<'_> {
        Self::all().filter(Self::with_key(key))
    }

    /// Create a select statement that returns all global variables with the
    /// decrypted value.
    pub(crate) fn all() -> All {
        global_variables::table.select(all_columns())
    }
}

/// Use this struct to create a new global variable.
#[derive(Debug, Insertable, AsChangeset)]
#[table_name = "global_variables"]
pub(crate) struct NewGlobalVariable<'a> {
    key: &'a str,
    value: pgp_sym_encrypt::HelperType<&'a str, &'static str>,
}

impl<'a> NewGlobalVariable<'a> {
    /// Initialize a new global variable.
    ///
    /// This function makes sure the eventual value stored in the database is
    /// encrypted.
    pub(crate) fn new(key: &'a str, value: &'a str) -> Self {
        Self {
            key,
            value: pgp_sym_encrypt(value, SERVER_SECRET.as_str()),
        }
    }

    /// Save the new global variable in the database.
    ///
    /// If an existing variable exists with the same key, this method will
    /// return an error. If you want to "upsert" a variable, use
    /// `create_or_update`.
    pub fn create(self, conn: &PgConnection) -> QueryResult<GlobalVariable> {
        use diesel::insert_into;

        insert_into(global_variables::table)
            .values(&self)
            .returning(all_columns())
            .get_result(conn)
    }

    /// Save the new global variable in the database, or update a variable
    /// matching the key.
    pub fn create_or_update(self, conn: &PgConnection) -> QueryResult<GlobalVariable> {
        use diesel::insert_into;

        insert_into(global_variables::table)
            .values(&self)
            .on_conflict(global_variables::key)
            .do_update()
            .set(&self)
            .returning(all_columns())
            .get_result(conn)
    }
}

type AllColumns = (
    global_variables::id,
    global_variables::key,
    pgp_sym_decrypt::HelperType<global_variables::value, &'static str>,
);

type All = diesel::dsl::Select<global_variables::table, AllColumns>;
type WithKey<'a> = diesel::dsl::Eq<global_variables::key, &'a str>;
type ByKey<'a> = diesel::dsl::Filter<All, WithKey<'a>>;

fn all_columns() -> AllColumns {
    (
        global_variables::id,
        global_variables::key,
        pgp_sym_decrypt(global_variables::value, SERVER_SECRET.as_str()),
    )
}

sql_function!(fn pgp_sym_encrypt(data: Text, secret: Text) -> Bytea);
sql_function!(fn pgp_sym_decrypt(data: Bytea, secret: Text) -> Text);
