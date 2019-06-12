#![allow(unused_import_braces, unreachable_pub)]

table! {
    pipelines (id) {
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
        pipeline_id -> Integer,
    }
}

table! {
    task_steps (id) {
        id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
        processor -> Jsonb,
        position -> Integer,
        started_at -> Nullable<Timestamp>,
        finished_at -> Nullable<Timestamp>,
        status -> crate::resources::TaskStepStatusMapping,
        output -> Nullable<Text>,
        task_id -> Integer,
    }
}

table! {
    tasks (id) {
        id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
        status -> crate::resources::TaskStatusMapping,
        pipeline_reference -> Nullable<Integer>,
    }
}

table! {
    variables (id) {
        id -> Integer,
        key -> Text,
        description -> Nullable<Text>,
        pipeline_id -> Integer,
    }
}

joinable!(steps -> pipelines (pipeline_id));
joinable!(task_steps -> tasks (task_id));
joinable!(tasks -> pipelines (pipeline_reference));
joinable!(variables -> pipelines (pipeline_id));

allow_tables_to_appear_in_same_query!(pipelines, steps, task_steps, tasks, variables);
