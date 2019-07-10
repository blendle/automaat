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
    jobs (id) {
        id -> Integer,
        name -> Text,
        description -> Nullable<Text>,
        status -> crate::resources::JobStatusMapping,
        pipeline_reference -> Nullable<Integer>,
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
        pipeline_id -> Integer,
    }
}

joinable!(steps -> pipelines (pipeline_id));
joinable!(job_steps -> jobs (job_id));
joinable!(jobs -> pipelines (pipeline_reference));
joinable!(variables -> pipelines (pipeline_id));

allow_tables_to_appear_in_same_query!(pipelines, steps, job_steps, jobs, variables);
