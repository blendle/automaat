use super::PipelineDetailsView;
use crate::resources::Pipeline;
use crate::utils::{document, element, element_child};
use typed_html::{dom::DOMTree, html, text};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::HtmlElement;

pub(crate) struct PipelinesView;

impl PipelinesView {
    pub(crate) fn update(pipelines: &[Pipeline]) {
        let results = match element("#results") {
            None => return,
            Some(el) => el,
        };

        results.set_inner_html("");

        for pipeline in pipelines {
            let el = document().create_element("div").expect("valid div element");
            el.set_inner_html(Self::html(pipeline).as_str());

            let trigger = match element_child(&el, "a.pipeline-trigger") {
                None => continue,
                Some(el) => el,
            };

            if let Ok(dyn_el) = trigger.dyn_into::<HtmlElement>() {
                let id = pipeline.id.to_owned();
                let show_pipeline_details: Closure<dyn Fn()> =
                    Closure::wrap(Box::new(move || PipelineDetailsView::show(id.clone())));

                dyn_el.set_onclick(Some(show_pipeline_details.as_ref().unchecked_ref()));
                let _ = results.append_child(&el).expect("successful append");

                show_pipeline_details.forget();
            }
        }
    }

    fn html(pipeline: &Pipeline) -> String {
        let dom: DOMTree<String> = html! {
        <div class="columns is-centered is-flex">
          <div
            data-id={ pipeline.id.as_str() }
            class="search-result column is-mobile is-one-third-desktop is-half-tablet"
            style="box-shadow: 0 2px 3px
            rgba(10,10,10,.1),0 0 0 1px rgba(10,10,10,.1); border-radius: 6px; margin: .75rem 0;"
          >
            <div class="columns is-mobile">
              <div
                class="column has-background-white-ter"
                style="border-radius: 6px 0 0 6px"
              >
                <div class="columns is-mobile">
                  <div class="column">
                    <h1 class="title is-6 has-text-centered is-uppercase">
                      { text!("{}", pipeline.name) }
                    </h1>
                  </div>
                </div>

                <div class="columns is-mobile">
                  <div class="column">
                    <p
                      class="bd-notification is-info has-text-weight-light has-text-grey is-size-6"
                      style="max-height: 100px; overflow: hidden"
                    >
                      { text!("{}", pipeline.description.as_ref().unwrap_or(&"".to_owned())) }
                    </p>
                  </div>
                </div>
              </div>

              <a
                href="#"
                tabindex="0"
                class="pipeline-trigger level column is-narrow has-background-primary"
                style="border-radius: 0 6px 6px 0; display: flex"
              >
                <div class="icon has-text-white is-size-4">
                  <i class="fas fa-play level-item"></i>
                </div>
              </a>
            </div>
          </div>
        </div>
        };

        dom.to_string()
    }
}
