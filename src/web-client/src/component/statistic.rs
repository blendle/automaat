//! A single statistic shown in the application header.

use dodrio::bumpalo::collections::string::String as BString;
use dodrio::{Node, Render, RenderContext};

/// The Statistic component.
pub(crate) struct Statistic {
    /// The name of the statistic shown in the header.
    name: &'static str,

    /// The optional integer value for the statistic.
    ///
    /// If this value is unset, an empty string is used instead. This can be
    /// used to lazy-load the statistic, without blocking the main UI from
    /// loading.
    value: Option<u32>,

    /// If the statistic is disabled, a grey `x` is shown instead of the
    /// statistic number.
    enabled: bool,
}

impl Statistic {
    /// Create a new Statistic component that is enabled.
    pub(crate) const fn new(name: &'static str, value: Option<u32>) -> Self {
        Self {
            name,
            value,
            enabled: true,
        }
    }

    /// Create a new Statistic component that is disabled.
    pub(crate) const fn disabled(name: &'static str) -> Self {
        Self {
            name,
            value: None,
            enabled: false,
        }
    }
}

impl Render for Statistic {
    fn render<'a>(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        use dodrio::builder::*;

        let mut stat = div(&cx).child(p(&cx).child(text(self.name)).finish());

        if self.enabled {
            let string = self.value.map_or(" ".to_owned(), |v| v.to_string());
            let value = BString::from_str_in(string.as_str(), cx.bump).into_bump_str();
            stat = stat.child(p(&cx).child(text(value)).finish());
        } else {
            stat = stat.child(p(&cx).child(text("")).finish())
        };

        div(&cx)
            .attr("class", "au-statistic")
            .bool_attr("disabled", !self.enabled)
            .child(stat.finish())
            .finish()
    }
}
