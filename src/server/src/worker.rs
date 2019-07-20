use crate::resources::Job;
use crate::State;
use diesel::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{error::Error, thread, time};

pub(crate) struct Worker {
    state: State,
}

pub(crate) enum Event {
    Done,
    NoPendingJob,
    ConnectionPoolError(diesel::r2d2::PoolError),
    DatabaseError(diesel::result::Error),
}

impl Worker {
    pub(crate) const fn new(state: State) -> Self {
        Self { state }
    }

    pub(crate) fn run_to_completion(mut self) -> Result<(), Box<dyn Error>> {
        let running = Arc::new(AtomicBool::new(true));
        let closer = running.clone();
        ctrlc::set_handler(move || closer.store(false, Ordering::SeqCst))?;

        while running.load(Ordering::SeqCst) {
            use Event::*;
            match self.run_single_job() {
                NoPendingJob => thread::sleep(time::Duration::from_millis(100)),
                Done => {}
                ConnectionPoolError(err) => return Err(err.into()),
                DatabaseError(err) => return Err(err.into()),
            };
        }

        Ok(())
    }

    pub(crate) fn run_single_job(&mut self) -> Event {
        use Event::*;

        let conn = match self.state.pool.get() {
            Ok(conn) => conn,
            Err(err) => return ConnectionPoolError(err),
        };

        let result = conn.transaction(|| {
            let mut job = match Job::find_next_unlocked_pending(&conn) {
                Ok(Some(job)) => job,
                Ok(None) => return Ok(NoPendingJob),
                Err(err) => return Err(err),
            };

            job.as_running(&conn)?
                .run(&conn)
                .or_else(|_| job.as_failed(&conn).map(|_| ()))
                .map(|_| Done)
        });

        match result {
            Ok(event) => event,
            Err(err) => DatabaseError(err),
        }
    }
}
