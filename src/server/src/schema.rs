#![allow(unused_import_braces, unreachable_pub)]

table! {
    tasks (id) {
        id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
    }
}

table! {
    steps (id) {
        id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
        processor -> Jsonb,
        position -> Integer,
        task_id -> Integer,
    }
}

table! {
    job_steps (id) {
        id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
        processor -> Jsonb,
        position -> Integer,
        started_at -> Nullable<Timestamp>,
        finished_at -> Nullable<Timestamp>,
        status -> crate::resources::JobStepStatusMapping,
        output -> Nullable<Text>,
        job_id -> Integer,
    }
}

table! {
    job_variables (id) {
        id -> Integer,
        key -> Text,
        value -> Bytea,
        job_id -> Integer,
    }
}

table! {
    jobs (id) {
        id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
        status -> crate::resources::JobStatusMapping,
        task_reference -> Nullable<Integer>,
    }
}

table! {
    variables (id) {
        id -> Integer,
        key -> Text,
        description -> Nullable<Text>,
        selection_constraint -> Nullable<Array<Text>>,
        default_value -> Nullable<Text>,
        example_value -> Nullable<Text>,
        task_id -> Integer,
    }
}

joinable!(steps -> tasks (task_id));
joinable!(job_steps -> jobs (job_id));
joinable!(job_variables -> jobs (job_id));
joinable!(jobs -> tasks (task_reference));
joinable!(variables -> tasks (task_id));

allow_tables_to_appear_in_same_query!(tasks, steps, job_steps, job_variables, jobs, variables);
