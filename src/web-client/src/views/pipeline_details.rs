use crate::resources::{CreateTaskFromPipeline, PipelineDetails, Task, TaskStatus, TaskStepStatus};
use crate::utils::{element, element_child, keyboard_event, window};
use crate::views::SearchBarView;
use comrak::{markdown_to_html, ComrakOptions};
use futures::prelude::*;
use std::collections::HashMap;
use typed_html::{dom::DOMTree, html, text, unsafe_text};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlElement, HtmlInputElement};

pub(crate) struct PipelineDetailsView;

const ENTER_KEY: u32 = 13;
const ESCAPE_KEY: u32 = 27;

impl PipelineDetailsView {
    pub(crate) fn hide() {
        // We need to re-attach the search bar shortcut, since we replace them
        // with the pipeline details shortcuts when we showed this view.
        SearchBarView::set_keyboard_shortcuts();

        match element("#pipeline-modal") {
            None => (),
            Some(el) => el.set_inner_html(""),
        }
    }

    pub(crate) fn add_task_status(task: &Task) {
        Self::loading(false);

        let (default_msg, class, title) = match task.status {
            TaskStatus::FAILED => (
                "The pipeline failed for unknown reasons.",
                "is-danger",
                "Failed!",
            ),
            TaskStatus::OK => ("The pipeline ran successfully.", "is-success", "Success!"),
            _ => (
                "The pipeline returned an unexpected status",
                "is-warning",
                "Unknown Status!",
            ),
        };

        let message = task
            .steps
            .as_ref()
            .and_then(|s| {
                s.iter()
                    .find(|s| s.status == TaskStepStatus::FAILED)
                    .or_else(|| s.last())
            })
            .and_then(|s| s.output.as_ref().map(String::as_str))
            .unwrap_or(default_msg);

        let message = markdown_to_html(message, &ComrakOptions::default());

        let msg: DOMTree<String> = html! {
            <article class={ format!("message {}", class).as_str() }>
                <div class="message-header">
                    <p>{ text!("{}", title) }</p>
                </div>
                <div class="message-body">
                    { unsafe_text!(message) }
                </div>
            </article>
        };

        if let Some(el) = element("#pipeline-modal #modal-messages") {
            el.set_inner_html(msg.to_string().as_str())
        }
    }

    pub(crate) fn loading(active: bool) {
        if let Some(el) = element("#pipeline-modal #modal-loading") {
            let classes = el.class_list();

            if active {
                classes.remove_1("is-hidden").expect("class removed");
            } else {
                classes.add_1("is-hidden").expect("class added");
            }
        }
    }

    pub(crate) fn remove_messages() {
        if let Some(el) = element("#pipeline-modal #modal-messages") {
            el.set_inner_html("");
        }
    }

    pub(crate) fn add_errors(errors: Vec<String>) {
        Self::loading(false);

        match element("#pipeline-modal") {
            None => return,
            Some(ref el) if el.children().length() == 0 => return,
            _ => (),
        };

        let errors: Vec<_> = errors
            .into_iter()
            .map(|error| markdown_to_html(error.as_str(), &ComrakOptions::default()))
            .map(|html| unsafe_text!("{}", html))
            .collect();

        let msg: DOMTree<String> = html! {
            <article class="message is-danger">
                <div class="message-header">
                <p>"Failure!"</p>
                </div>
                <div class="message-body">
                    { errors.into_iter() }
                </div>
            </article>
        };

        if let Some(el) = element("#pipeline-modal #modal-messages") {
            el.set_inner_html(msg.to_string().as_str())
        }
    }

    pub(crate) fn run_pipeline(pipeline_id: String) {
        let mut variables = HashMap::default();

        if let Some(el) = element("#pipeline-modal #pipeline-details-variables") {
            let inputs = el.query_selector_all("input").expect("valid selector");

            (0..(inputs.length())).for_each(|i| {
                if let Some(input) = inputs.item(i) {
                    if let Some(input) = JsCast::dyn_ref::<HtmlInputElement>(&input) {
                        if let Some(key) = input.get_attribute("data-key") {
                            let _ = variables.insert(key, input.value());
                        }
                    }
                }
            });

            spawn_local(CreateTaskFromPipeline::post(pipeline_id, variables));

            Self::remove_messages();
            Self::loading(true);
        }
    }

    pub(crate) fn show(pipeline_id: String) {
        spawn_local(PipelineDetails::fetch(pipeline_id).and_then(|pipeline| {
            let pipeline_details = match element("#pipeline-modal") {
                None => return futures::future::err(()),
                Some(el) => el,
            };

            pipeline_details.set_inner_html(Self::html(&pipeline).as_str());

            if let Some(input) = element_child(&pipeline_details, "input:first-of-type") {
                if let Some(dyn_input) = input.dyn_ref::<HtmlInputElement>() {
                    dyn_input.focus().expect("focussed");
                }
            }

            let hide_pipeline_details: Closure<dyn Fn()> = Closure::wrap(Box::new(Self::hide));

            // Add `onclick` handler to hide pipeline details view when clicking
            // on the "faded" background around the modal window.
            if let Some(el) = element("#pipeline-modal div.modal-background") {
                if let Some(dyn_el) = el.dyn_ref::<HtmlElement>() {
                    dyn_el.set_onclick(Some(hide_pipeline_details.as_ref().unchecked_ref()));
                }
            }

            // Add `onclick` handler to the "cancel" button in the pipeline
            // details view to hide the view.
            if let Some(el) = element("#pipeline-modal button#modal-cancel-button") {
                if let Some(dyn_el) = el.dyn_ref::<HtmlElement>() {
                    dyn_el.set_onclick(Some(hide_pipeline_details.as_ref().unchecked_ref()));
                }
            }
            hide_pipeline_details.forget();

            // Add `onclick` handler to the "run" button in the pipeline
            // details view to run the pipeline.
            if let Some(el) = element("#pipeline-modal button#modal-run-button") {
                if let Some(dyn_el) = el.dyn_ref::<HtmlElement>() {
                    let run_pipeline: Closure<dyn Fn()> =
                        Closure::wrap(Box::new(move || Self::run_pipeline(pipeline.id.clone())));

                    dyn_el.set_onclick(Some(run_pipeline.as_ref().unchecked_ref()));
                    run_pipeline.forget();
                }
            }

            Self::set_keyboard_shortcuts();
            futures::future::ok(())
        }));
    }

    fn set_keyboard_shortcuts() {
        let shortcuts: Closure<dyn Fn(_)> = Closure::wrap(Box::new(move |e: Event| {
            match keyboard_event(&e) {
                Some(ESCAPE_KEY) => Self::hide(),
                Some(ENTER_KEY) => Self::run(),
                _ => return,
            };

            e.prevent_default();
        }));

        window().set_onkeydown(Some(shortcuts.as_ref().unchecked_ref()));
        shortcuts.forget();
    }

    fn run() {
        if let Some(el) = element("#pipeline-modal button#modal-run-button") {
            if let Some(dyn_el) = el.dyn_ref::<HtmlElement>() {
                dyn_el.click();
            }
        }
    }

    fn html(details: &PipelineDetails) -> String {
        let empty = vec![];
        let variables = details.variables.as_ref().unwrap_or(&empty);

        let dom: DOMTree<String> = html! {
            <div
              id="pipeline-details"
              data-id={ details.id.as_str() }
              class="modal is-active"
            >
              <div class="modal-background"></div>
              <div id="pipeline-details-container" class="modal-card">

                  <header class="modal-card-head has-background-primary">
                    <p class="modal-card-title has-text-white has-text-weight-semibold is-uppercase">
                      { text!("{}", details.name) }
                    </p>
                  </header>

                  <section class="modal-card-body has-background-white-ter">
                    <div class="container">
                      <div class="columns is-centered">
                        <div class="column">
                          <p class="content">
                            { text!("{}", details.description.as_ref().unwrap_or(&"".to_owned())) }
                          </p>

                          <div id="pipeline-details-variables">

                            { variables.iter().map(|var| { html! {

                            <div class="columns is-gapless">
                              <div class="column is-one-quarter">
                                <div class="field-label is-normal">
                                  <label class="label">{ text!("{}", var.key) }</label>
                                </div>
                              </div>
                              <div class="column">
                                <div class="field">
                                  <div class="control">
                                    <input
                                      class="input"
                                      type="text"
                                      data-key={ var.key.as_str() }
                                      placeholder=""
                                    />
                                  </div>
                                  <p class="help">
                                    { text!("{}", var.description.as_ref().unwrap_or(&"".to_owned()).as_str()) }
                                  </p>
                                </div>
                              </div>
                            </div>

                            } }) }

                          </div>
                        </div>
                      </div>

                      <div class="columns is-centered">
                        <div class="column">
                          <div id="modal-loading" class="content is-hidden">
                            <progress class="progress is-large is-info"></progress>
                          </div>
                        </div>
                      </div>

                      <div class="columns is-centered">
                        <div class="column">
                          <div id="modal-messages" class="content"></div>
                        </div>
                      </div>
                    </div>
                  </section>

                  <footer class="modal-card-foot">
                    <button id="modal-cancel-button" class="button is-medium is-outlined">
                      <span>"Cancel"</span>
                    </button>

                    <button id="modal-run-button" class="button is-medium is-fullwidth is-info">
                      <span>"Run Pipeline"</span>
                      <span class="icon">
                        <i class="fas fa-check"></i>
                      </span>
                    </button>
                  </footer>

              </div>
            </div>
        };

        dom.to_string()
    }
}
