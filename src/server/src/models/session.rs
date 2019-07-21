use crate::schema::sessions;
use diesel::prelude::*;
use uuid::Uuid;

/// The model representing a session stored in the database.
#[derive(Clone, Copy, Debug, Identifiable, Queryable)]
#[table_name = "sessions"]
pub(crate) struct Session {
    pub(crate) id: i32,
    pub(crate) token: Uuid,
}

impl Session {
    pub(crate) fn find_by_token(token: Uuid, conn: &PgConnection) -> QueryResult<Self> {
        sessions::table
            .filter(sessions::token.eq(token))
            .first(conn)
    }

    /// Create a new session in the database.
    ///
    /// All values will be set to their defaults, including generating a session
    /// token in the database.
    pub(crate) fn create(conn: &PgConnection) -> QueryResult<Self> {
        diesel::insert_into(sessions::table)
            .default_values()
            .get_result(conn)
    }
}
