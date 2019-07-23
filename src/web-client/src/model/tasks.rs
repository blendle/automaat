//! A collection of cached tasks, and any application session specific
//! configuration tied to that list (such as visibility filters, etc...).

use crate::graphql::search_tasks::SearchTasksTasks;
use crate::model::task::{Id, Task};
use dodrio::{RootRender, VdomWeak};
use futures::future::Future;
use std::collections::HashMap;
use std::convert::From;

/// The tasks model.
#[derive(Clone, Debug, Default)]
pub(crate) struct Tasks {
    /// A set of tasks known to this task set. This includes all tasks ever
    /// searched for in the current session, so it should be viewed as a cache
    /// of tasks.
    ///
    /// The `filtered_task_ids` is used to keep a record of tasks that should be
    /// shown in the UI based on the active search query.
    tasks: HashMap<Id, Task>,

    /// A list of active tasks.
    ///
    /// An active task is one that is either open, or was open while navigating
    /// towards a different task.
    ///
    /// You can think of this list as "breadcrumbs" of tasks that have been
    /// visited starting from the first task opened from the home screen.
    ///
    /// When the home page is active, this list is emptied.
    ///
    /// The list is ordered from oldest to newest active task, meaning the last
    /// task in the list is the currently active task.
    active_task_ids: Vec<Id>,

    /// A list of Ids that represents a subset of stored tasks to be shown in
    /// the search view.
    filtered_task_ids: Option<Vec<Id>>,
}

impl Tasks {
    /// Set an existing task as the "active task" (the one being viewed in the
    /// UI), by providing the task ID.
    ///
    /// This method returns an `Err` if there is no task matching the provided
    /// ID.
    ///
    /// If the provided task ID matches the last active task ID, activation is
    /// skipped, and this method is a no-op.
    pub(crate) fn activate_task(&mut self, id: Id) -> Result<&Task, ()> {
        if let Some(task) = self.tasks.get(&id) {
            if let Some(active_task) = self.active_task() {
                if active_task.id() == id {
                    return Ok(task);
                }
            }

            self.active_task_ids.push(id);
            Ok(task)
        } else {
            Err(())
        }
    }

    /// Get a reference to the active task, if any.
    pub(crate) fn active_task(&self) -> Option<&Task> {
        match self.active_task_ids.last() {
            None => None,
            Some(id) => self.tasks.get(id),
        }
    }

    /// Get a mutable reference to the active task, if any.
    pub(crate) fn active_task_mut(&mut self) -> Option<&mut Task> {
        match self.active_task_ids.last() {
            None => None,
            Some(id) => self.tasks.get_mut(id),
        }
    }

    /// Add a new task to the list of tasks.
    pub(crate) fn add(&mut self, task: Task) {
        let _ = self.tasks.insert(task.id(), task);
    }

    /// Take a vector of tasks and add any that are still missing, or update existing ones that
    /// have the same amount (but possibly outdated) information stored in them.
    pub(crate) fn append(&mut self, tasks: Vec<Task>) {
        for task in tasks {
            if let Some(existing) = self.get_mut(&task.id()) {
                if existing.variables().is_none() {
                    *existing = task
                }
            } else {
                self.add(task)
            }
        }
    }

    /// Check if a given task ID exists in the cache.
    pub(crate) fn contains(&self, id: &Id) -> bool {
        self.tasks.contains_key(id)
    }

    /// Unset any active task.
    ///
    /// This also deactivates any jobs attached to this task.
    pub(crate) fn disable_active_task(&mut self) {
        if let Some(task) = self.active_task_mut() {
            task.deactivate();
        }

        let _ = self.active_task_ids.pop();
    }

    /// Disable all active tasks, creating an empty stack.
    pub(crate) fn disable_all_active_tasks(&mut self) {
        while let Some(task) = self.active_task_mut() {
            task.deactivate();
            let _ = self.active_task_ids.pop();
        }
    }

    /// Sets the active task filter, based on a set of provided task IDs.
    ///
    /// The provided IDs are filtered down to a set of IDs that are known to
    /// this task set. No error is returned if one or more IDs are unknown, but
    /// these IDs are ignored.
    pub(crate) fn filter_tasks(&mut self, ids: Vec<Id>) {
        let ids = ids.into_iter().filter(|i| self.contains(i)).collect();

        self.filtered_task_ids = Some(ids);
    }

    /// Returns the set of actively filtered tasks. This filter can be set for
    /// any reason, but right now it is set by the search action on the
    /// controller.
    pub(crate) fn filtered_tasks(&self) -> Vec<&Task> {
        match &self.filtered_task_ids {
            None => self.tasks.values().collect(),
            Some(ids) => self
                .tasks
                .values()
                .filter(|t| ids.contains(&t.id()))
                .collect(),
        }
    }

    /// Get a reference to a task, based on its ID, if the task is known to the
    /// task set.
    pub(crate) fn get(&self, id: &Id) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// Get a mutable reference to a task, based on its ID, if the task is known
    /// to the task set.
    pub(crate) fn get_mut(&mut self, id: &Id) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }
}

impl<'a> IntoIterator for &'a Tasks {
    type Item = &'a Task;
    type IntoIter = std::collections::hash_map::Values<'a, Id, Task>;

    fn into_iter(self) -> Self::IntoIter {
        self.tasks.values()
    }
}

impl From<Vec<SearchTasksTasks>> for Tasks {
    fn from(results: Vec<SearchTasksTasks>) -> Self {
        let mut tasks = HashMap::new();
        let vec: Vec<Task> = results.into_iter().map(Into::into).collect();

        for task in vec {
            let _ = tasks.insert(task.id().clone(), task);
        }

        Self {
            tasks,
            active_task_ids: vec![],
            filtered_task_ids: None,
        }
    }
}

/// The actions a controller has to implement to bridge between the UI and the
/// model.
pub(crate) trait Actions {
    /// Search for tasks, based on their name or description.
    ///
    /// The resulting tasks should be added to the `Tasks` model for future use.
    fn search(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
        query: String,
    ) -> Box<dyn Future<Item = (), Error = ()>>;
}
