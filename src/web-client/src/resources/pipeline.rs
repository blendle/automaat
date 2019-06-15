use self::fetch_pipeline_details::FetchPipelineDetailsPipeline;
use core::ops::Deref;
use futures::prelude::*;
use graphql_client::GraphQLQuery;
use std::convert::TryFrom;
use url_serde::SerdeUrl as Url;

pub(crate) use self::fetch_pipelines::FetchPipelinesPipelines as Pipeline;
pub(crate) use self::fetch_pipelines::SearchPipelineInput;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/fetch_pipelines.graphql"
)]
pub(crate) struct FetchPipelines;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/fetch_pipeline_details.graphql"
)]
pub(crate) struct FetchPipelineDetails;

pub(crate) struct Pipelines;

impl Pipelines {
    pub(crate) fn fetch(
        search: Option<SearchPipelineInput>,
    ) -> impl Future<Item = Vec<Pipeline>, Error = ()> {
        let client = graphql_client::web::Client::new("/graphql");
        client
            .call(FetchPipelines, fetch_pipelines::Variables { search })
            .then(|response| match response {
                Ok(response) => match response.data {
                    Some(data) => Ok(data.pipelines),
                    None => Err(()),
                },
                Err(_) => Err(()),
            })
    }
}

pub(crate) struct PipelineDetails(FetchPipelineDetailsPipeline);

impl Deref for PipelineDetails {
    type Target = FetchPipelineDetailsPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PipelineDetails {
    pub(crate) fn fetch(id: String) -> impl Future<Item = Self, Error = ()> {
        use fetch_pipeline_details::Variables;

        let client = graphql_client::web::Client::new("/graphql");
        client
            .call(FetchPipelineDetails, Variables { id })
            .then(|response| {
                response
                    .map(|r| r.data.map(|d| d.pipeline))
                    .map(|p| Self(p.expect("pipeline exists").unwrap()))
            })
            .map_err(|_| ())
    }
}

impl TryFrom<Option<String>> for SearchPipelineInput {
    type Error = ();

    fn try_from(query: Option<String>) -> Result<Self, Self::Error> {
        match query {
            None => Err(()),
            Some(q) => Ok(Self {
                name: Some(q.clone()),
                description: Some(q),
            }),
        }
    }
}
