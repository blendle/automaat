//! The representation of a single variable belonging to a task.
//!
//! This component shows the name of a variable, along with the appropriate
//! input field, depending on the variable properties (such as if it's required,
//! if the types of values are constraint, etc.).

use crate::model::variable::{self, ValueAdvertiser};
use crate::router::Route;
use crate::utils;
use dodrio::bumpalo::{collections::string::String, format, Bump};
use dodrio::{Node, Render, RenderContext};
use wasm_bindgen::UnwrapThrowExt;

/// The `Variable` component.
pub(crate) struct Variable<'a> {
    /// A reference to the variable for which the component is rendered.
    variable: &'a variable::Variable<'a>,

    /// A variable can have a pre-existing value if the task the variable
    /// belongs to has a job attached to it.
    ///
    /// If this is the case, it means the task already ran with a set of values
    /// provided by the person running that task. We want to preserve those
    /// values, to prevent the bad UX of reverting any provided values back to
    /// their defaults as soon as the task is run.
    existing_value: Option<&'a str>,
}

impl<'a> Variable<'a> {
    /// Returns the value of the variable.
    ///
    /// There are four possible value types returned by this method:
    ///
    /// * A pre-existing value (see `existing_value`).
    /// * A value set via the location query string.
    /// * The default variable value, as provided by the server.
    /// * An empty string, if no pre-existing or default value exists.
    fn value<'b, B>(&self, bump: B) -> &'b str
    where
        B: Into<&'b Bump>,
    {
        let value = utils::get_location_query(self.variable.key());
        let value = match self.existing_value {
            None => match value.as_ref() {
                None => self.variable.default_value().unwrap_or(""),
                Some(value) => value.as_str(),
            },
            Some(value) => value,
        };

        String::from_str_in(value, bump.into()).into_bump_str()
    }

    /// Returns the optional placeholder value of a variable.
    ///
    /// The placeholder is based on the example value set by the server for this
    /// variable.
    ///
    /// It is only used in the "text input" field type, as that's the only field
    /// type that allows free-form input, and would thus benefit from an
    /// example.
    fn placeholder<'b, B>(&self, bump: B) -> Option<&'b str>
    where
        B: Into<&'b Bump>,
    {
        match self.variable.example_value() {
            None => None,
            Some(value) => Some(format!(in bump.into(), "e.g. \"{}\"", value).into_bump_str()),
        }
    }
}

/// The trait implemented by this component to render all its views.
trait Views<'b> {
    /// The label/description of the variable.
    fn label(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// A check box field for when the variable can only contain a single value.
    fn checkbox(&self, cx: &mut RenderContext<'b>, selection: &[&str]) -> Node<'b>;

    /// A set of radio buttons for when the variable can contain exactly two
    /// values.
    fn radio(&self, cx: &mut RenderContext<'b>, selection: &[&str]) -> Node<'b>;

    /// A select field for when the variable can contain three or more values.
    fn select(&self, cx: &mut RenderContext<'b>, selection: &[&str]) -> Node<'b>;

    /// A free-form text input field for when there are no value constraints
    /// imposed on a variable.
    fn input(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// A variable field, which contains a label, and one of the defined field
    /// types.
    fn field(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// Any extra details shown below the variable field.
    fn details(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// One or more value advertisers presented as a means to provide the value
    /// for this variable.
    fn value_advertisers(
        &self,
        cx: &mut RenderContext<'b>,
        adverts: Vec<ValueAdvertiser<'_>>,
    ) -> Node<'b>;
}

impl<'a, 'b> Views<'b> for Variable<'a> {
    fn label(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let key = String::from_str_in(self.variable.key(), cx.bump).into_bump_str();
        let label = label(&cx).child(text(key)).finish();

        div(&cx)
            .attr("class", "variable-label")
            .child(div(&cx).child(div(&cx).child(label).finish()).finish())
            .finish()
    }

    fn checkbox(&self, cx: &mut RenderContext<'b>, selection: &[&str]) -> Node<'b> {
        use dodrio::builder::*;

        let key = String::from_str_in(self.variable.key(), cx.bump).into_bump_str();
        let value = selection.get(0).unwrap_throw();
        let value = String::from_str_in(value, cx.bump).into_bump_str();

        let label = label(&cx)
            .child(
                input(&cx)
                    .bool_attr("checked", true)
                    .bool_attr("disabled", true)
                    .attr("type", "checkbox")
                    .finish(),
            )
            .child(
                input(&cx)
                    .attr("type", "hidden")
                    .attr("name", key)
                    .attr("value", value)
                    .finish(),
            )
            .child(text(" "))
            .child(text(value))
            .finish();

        div(&cx)
            .child(
                div(&cx)
                    .attr("class", "variable-checkbox")
                    .child(label)
                    .finish(),
            )
            .finish()
    }

    fn radio(&self, cx: &mut RenderContext<'b>, selection: &[&str]) -> Node<'b> {
        use dodrio::builder::*;

        let key = String::from_str_in(self.variable.key(), cx.bump).into_bump_str();

        let labels: Vec<_> = selection
            .iter()
            .map(|v| String::from_str_in(v, cx.bump).into_bump_str())
            .map(|v| {
                label(&cx)
                    .child(
                        input(&cx)
                            .bool_attr("checked", self.value(cx.bump) == v)
                            .attr("type", "radio")
                            .attr("value", v)
                            .attr("name", key)
                            .on("click", move |_root, _vdom, event| {
                                let target = event.target().unwrap_throw();
                                utils::input_to_location_query(target).unwrap_throw();
                            })
                            .finish(),
                    )
                    .child(text(" "))
                    .child(text(v))
                    .finish()
            })
            .collect();

        div(&cx)
            .child(
                div(&cx)
                    .attr("class", "variable-radio")
                    .children(labels)
                    .finish(),
            )
            .finish()
    }

    fn select(&self, cx: &mut RenderContext<'b>, selection: &[&str]) -> Node<'b> {
        use dodrio::builder::*;

        let key = String::from_str_in(self.variable.key(), cx.bump).into_bump_str();
        let options: Vec<_> = selection
            .iter()
            .map(|v| String::from_str_in(v, cx.bump).into_bump_str())
            .map(|v| {
                option(&cx)
                    .bool_attr("selected", self.value(cx.bump) == v)
                    .child(text(v))
                    .finish()
            })
            .collect();

        div(&cx)
            .child(
                div(&cx)
                    .attr("class", "variable-select")
                    .child(
                        select(&cx)
                            .attr("name", key)
                            .attr("aria-label", key)
                            .children(options)
                            .on("change", move |_root, _vdom, event| {
                                let target = event.target().unwrap_throw();
                                utils::input_to_location_query(target).unwrap_throw();
                            })
                            .finish(),
                    )
                    .finish(),
            )
            .finish()
    }

    fn input(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let key = String::from_str_in(self.variable.key(), cx.bump).into_bump_str();
        let mut attributes = vec![
            attr("type", "text"),
            attr("name", key),
            attr("aria-label", key),
            attr("value", self.value(cx.bump)),
        ];

        if let Some(value) = self.placeholder(cx.bump) {
            attributes.push(attr("placeholder", value))
        };

        let input = input(&cx)
            .attributes(attributes)
            .on("input", move |_root, _vdom, event| {
                let target = event.target().unwrap_throw();
                utils::input_to_location_query(target).unwrap_throw();
            })
            .finish();

        div(&cx).child(input).finish()
    }

    fn field(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let input = match &self.variable.selection_constraint() {
            Some(selection) if selection.len() == 1 => self.checkbox(cx, selection),
            Some(selection) if selection.len() <= 2 => self.radio(cx, selection),
            Some(selection) => self.select(cx, selection),
            None => self.input(cx),
        };

        div(&cx)
            .child(
                div(&cx)
                    .attr("class", "variable-field")
                    .children([input, self.details(cx)])
                    .finish(),
            )
            .finish()
    }

    fn details(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let description = String::from_str_in(self.variable.description(), cx.bump).into_bump_str();
        let mut node = div(&cx)
            .attr("class", "variable-details")
            .child(p(&cx).child(text(description)).finish());

        let adverts = self.variable.value_advertisers();
        if self.variable.selection_constraint().is_none() && !adverts.is_empty() {
            node = node.child(self.value_advertisers(cx, adverts));
        }

        node.finish()
    }

    fn value_advertisers(
        &self,
        cx: &mut RenderContext<'b>,
        adverts: Vec<ValueAdvertiser<'_>>,
    ) -> Node<'b> {
        use dodrio::builder::*;

        let details = |advert: &ValueAdvertiser<'_>| {
            let name = String::from_str_in(advert.name, cx.bump).into_bump_str();
            let description = match advert.description {
                None => None,
                Some(string) => Some(String::from_str_in(string, cx.bump).into_bump_str()),
            };

            let route = Route::Task(advert.task_id.clone());
            let url = format!(in cx.bump, "{}", route).into_bump_str();

            (name, description, url)
        };

        let icon = span(&cx)
            .attr("class", "info")
            .child(i(&cx).finish())
            .finish();

        let mut content = vec![icon];

        if adverts.len() == 1 {
            let (name, _, url) = details(adverts.get(0).unwrap_throw());

            content.extend_from_slice(&[
                text("The"),
                a(&cx).attr("href", url).child(text(name)).finish(),
                text("task can provide this value."),
            ]);
        } else {
            let mut items = vec![];
            for advert in &adverts {
                let (name, description, url) = details(advert);

                items.push(a(&cx).attr("href", url).child(text(name)).finish());

                if let Some(description) = description {
                    items.push(p(&cx).child(text(description)).finish());
                }

                items.push(hr(&cx).finish());
            }
            items.truncate(items.len() - 1);

            let trigger = div(&cx)
                .child(a(&cx).child(text("multiple tasks")).finish())
                .finish();

            let menu = div(&cx)
                .attr("role", "menu")
                .attr("class", "dropdown-menu")
                .child(div(&cx).children(items).finish())
                .finish();

            let dropdown = div(&cx)
                .attr("class", "menu")
                .children([trigger, menu])
                .finish();

            content.extend_from_slice(&[
                text("There are"),
                dropdown,
                text("that can provide this value."),
            ]);
        };

        div(&cx)
            .attr("class", "variable-advertisers")
            .child(span(&cx).children(content).finish())
            .finish()
    }
}

impl<'a> Render for Variable<'a> {
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        div(&cx)
            .attr("class", "variable")
            .children([self.label(cx), self.field(cx)])
            .finish()
    }
}

impl<'a> From<(&'a variable::Variable<'a>, Option<&'a str>)> for Variable<'a> {
    fn from((variable, existing_value): (&'a variable::Variable<'a>, Option<&'a str>)) -> Self {
        Self {
            variable,
            existing_value,
        }
    }
}
