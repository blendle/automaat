use crate::resources::PipelineVariable;
use typed_html::elements::FlowContent;
use typed_html::{html, text};

pub(crate) struct VariableInputView;

impl VariableInputView {
    pub(crate) fn html(var: &PipelineVariable) -> Box<dyn FlowContent<String>> {
        let key = var.key.as_str();
        let description = var.description.as_ref().map(String::as_str);

        let field = match var.constraints.selection {
            Some(ref selection) if selection.len() == 1 => {
                Self::checkbox_element(key, selection.get(0).unwrap())
            }
            Some(ref selection) if selection.len() <= 2 => Self::radio_elements(key, selection),
            Some(ref selection) => Self::select_element(key, selection),
            None => Self::field_element(key),
        };

        html! {
            <div class="columns is-gapless">
                <div class="column is-one-quarter">
                    <div class="field-label is-normal">
                        <label class="label">{ text!("{}", key) }</label>
                    </div>
                </div>
                <div class="column">
                    <div class="field">
                        { field }
                        <p class="help">
                            { text!("{}", description.unwrap_or("")) }
                        </p>
                    </div>
                </div>
            </div>
        }
    }

    fn select_element(key: &str, selection: &[String]) -> Box<dyn FlowContent<String>> {
        html! {
            <div class="control">
                <div class="select is-normal is-fullwidth">
                    <select class="pipeline-variable" data-key={ key }>
                        { selection.iter().map(|v| html!{
                            <option>{ text!("{}", v) }</option>
                        }) }
                    </select>
                </div>
            </div>
        }
    }

    fn checkbox_element(key: &str, value: &str) -> Box<dyn FlowContent<String>> {
        html! {
            <div class="control is-size-5">
                <label class="checkbox is-size-6">
                    <input
                        class="pipeline-variable"
                        type="checkbox"
                        checked=true
                        disabled=true
                        value={ value }
                        data-key={ key }
                    />
                    { text!(" {}", value) }
                </label>
            </div>
        }
    }

    fn radio_elements(key: &str, selection: &[String]) -> Box<dyn FlowContent<String>> {
        html! {
            <div class="control is-size-5">
                { selection.iter().map(|v| html!{
                    <label class="radio is-size-6">
                        <input
                            class="pipeline-variable"
                            type="radio"
                            value={ v.as_str() }
                            data-key={ key }
                            name={ crate::utils::format_id_from_str(key).as_str() }
                        />
                        { text!(" {}", v) }
                    </label>
                }) }
            </div>
        }
    }

    fn field_element(key: &str) -> Box<dyn FlowContent<String>> {
        html! {
            <div class="control">
                <input
                    class="input pipeline-variable"
                    type="text"
                    data-key={ key }
                    placeholder=""
                />
            </div>
        }
    }
}
