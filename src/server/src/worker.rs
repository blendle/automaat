use crate::resources::Job;
use diesel::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{env, error::Error, thread, time};

pub(crate) struct Worker {
    conn: PgConnection,
}

pub(crate) enum Event {
    Done,
    NoPendingJob,
    DatabaseError(diesel::result::Error),
}

impl Worker {
    /// Create a new worker instance.
    pub(crate) fn from_environment() -> Result<Self, Box<dyn Error>> {
        let database_url = env::var("DATABASE_URL")?;
        let conn = PgConnection::establish(&database_url)?;

        crate::embedded_migrations::run(&conn)?;

        Ok(Self { conn })
    }

    /// Start polling for pending jobs and run them to completion.
    ///
    /// This method blocks until a Unix `SIGINT` or `SIGTERM` signal is
    /// received. When any of these signals are received, any running job runs
    /// to completion, before the method returns.
    pub(crate) fn run_to_completion(self) -> Result<(), Box<dyn Error>> {
        let running = Arc::new(AtomicBool::new(true));
        let closer = running.clone();
        ctrlc::set_handler(move || closer.store(false, Ordering::SeqCst))?;

        while running.load(Ordering::SeqCst) {
            use Event::*;
            match self.run_single_job() {
                NoPendingJob => thread::sleep(time::Duration::from_millis(100)),
                Done => {}
                DatabaseError(err) => return Err(err.into()),
            };
        }

        Ok(())
    }

    /// Find a pending job in the database, and run it to completion.
    pub(crate) fn run_single_job(&self) -> Event {
        use Event::*;

        let result = self.conn.transaction(|| {
            let mut job = match Job::find_next_unlocked_pending(&self.conn) {
                Ok(Some(job)) => job,
                Ok(None) => return Ok(NoPendingJob),
                Err(err) => return Err(err),
            };

            job.as_running(&self.conn)?
                .run(&self.conn)
                .or_else(|_| job.as_failed(&self.conn).map(|_| ()))
                .map(|_| Done)
        });

        match result {
            Ok(event) => event,
            Err(err) => DatabaseError(err),
        }
    }
}
