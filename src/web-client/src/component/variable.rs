//! The representation of a single variable belonging to a task.
//!
//! This component shows the name of a variable, along with the appropriate
//! input field, depending on the variable properties (such as if it's required,
//! if the types of values are constraint, etc.).

use crate::model::variable;
use dodrio::bumpalo::{collections::string::String, Bump};
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
    /// There are three possible value types returned by this method:
    ///
    /// * A pre-existing value (see `existing_value`).
    /// * The default variable value, as provided by the server.
    /// * An empty string, if no pre-existing or default value exists.
    fn value<'b, B>(&self, bump: B) -> &'b str
    where
        B: Into<&'b Bump>,
    {
        let value = match self.existing_value {
            None => self.variable.default_value().unwrap_or(""),
            Some(value) => value,
        };

        String::from_str_in(value, bump.into()).into_bump_str()
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
                            .finish(),
                    )
                    .finish(),
            )
            .finish()
    }

    fn input(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let key = String::from_str_in(self.variable.key(), cx.bump).into_bump_str();
        let attributes = [
            attr("type", "text"),
            attr("name", key),
            attr("aria-label", key),
            attr("placeholder", ""),
            attr("value", self.value(cx.bump)),
        ];

        div(&cx)
            .child(input(&cx).attributes(attributes).finish())
            .finish()
    }

    fn field(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let input = match &self.variable.selection_constraint() {
            Some(selection) if selection.len() == 1 => self.checkbox(cx, selection),
            Some(selection) if selection.len() <= 2 => self.radio(cx, selection),
            Some(selection) => self.select(cx, selection),
            None => self.input(cx),
        };

        let description = String::from_str_in(self.variable.description(), cx.bump).into_bump_str();

        div(&cx)
            .child(
                div(&cx)
                    .attr("class", "variable-field")
                    .children([input, p(&cx).child(text(description)).finish()])
                    .finish(),
            )
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
