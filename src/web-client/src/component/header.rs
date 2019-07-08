//! The header of the application.
//!
//! The header contains a set of global application statistics, and the logo.

use crate::component::Statistic;
use crate::model::statistics::Statistics;
use dodrio::{Node, Render, RenderContext};
use std::cell::Ref;

/// The Header component.
pub(crate) struct Header<'a> {
    /// A reference to the application statistics state.
    stats: Ref<'a, Statistics>,
}

impl<'a> Header<'a> {
    /// Create a new Header component, based on a set of existing statistics.
    pub(crate) const fn new(stats: Ref<'a, Statistics>) -> Self {
        Self { stats }
    }
}

impl<'a> Render for Header<'a> {
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let logo = img(&cx)
            .attr("src", "img/logo.svg")
            .attr("style", "height:100px")
            .attr("alt", "Automaat logo")
            .finish();

        div(&cx)
            .attr("class", "au-header")
            .children([
                Statistic::new("tasks", self.stats.total_tasks).render(cx),
                Statistic::disabled("scheduled").render(cx),
                div(&cx).child(logo).finish(),
                Statistic::new("running", self.stats.running_jobs).render(cx),
                Statistic::new("failed", self.stats.failed_jobs).render(cx),
            ])
            .finish()
    }
}
