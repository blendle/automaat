use crate::schema::variable_advertisements;
use diesel::prelude::*;

/// The model representing a variable advertisement stored in the database.
#[derive(Debug, Identifiable, Queryable)]
pub(crate) struct VariableAdvertisement {
    pub(crate) id: i32,
    pub(crate) key: String,
    pub(crate) step_id: i32,
}

impl VariableAdvertisement {
    /// Add a key filter to the variable advertisement query.
    pub(crate) fn with_key(key: &str) -> WithKey<'_> {
        variable_advertisements::key.eq(key)
    }

    /// Build a query that filters variable advertisements, based on the
    /// provided key.
    pub(crate) fn by_key(key: &str) -> ByKey<'_> {
        variable_advertisements::table.filter(Self::with_key(key))
    }
}

/// Use this struct to create a new variable advertisement.
#[derive(Debug, Insertable, AsChangeset)]
#[table_name = "variable_advertisements"]
pub(crate) struct NewVariableAdvertisement<'a> {
    key: &'a str,
    step_id: i32,
}

impl<'a> NewVariableAdvertisement<'a> {
    /// Initialize a new variable advertisement.
    pub(crate) const fn new(key: &'a str, step_id: i32) -> Self {
        Self { key, step_id }
    }

    /// Save or update the variable advertisement in the database.
    pub(crate) fn create_or_update(
        self,
        conn: &PgConnection,
    ) -> QueryResult<VariableAdvertisement> {
        diesel::insert_into(variable_advertisements::table)
            .values(&self)
            .on_conflict(variable_advertisements::step_id)
            .do_update()
            .set(&self)
            .get_result(conn)
    }
}

type WithKey<'a> = diesel::dsl::Eq<variable_advertisements::key, &'a str>;
type ByKey<'a> = diesel::dsl::Filter<variable_advertisements::table, WithKey<'a>>;
