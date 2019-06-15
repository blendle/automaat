use super::PipelinesView;
use crate::resources::Pipelines;
use crate::utils::{element, element_is_active, keyboard_event, window};
use futures::prelude::*;
use std::convert::TryInto;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlInputElement};

const ESCAPE_KEY: u32 = 27;
const F_KEY: u32 = 70;

pub(crate) struct SearchBarView;

impl SearchBarView {
    pub(crate) fn init() {
        let input = match Self::input() {
            None => return,
            Some(el) => el,
        };

        let search_action: Closure<dyn Fn()> =
            Closure::wrap(Box::new(|| spawn_local(Self::search_pipelines())));

        input.set_oninput(Some(search_action.as_ref().unchecked_ref()));
        search_action.forget();

        Self::focus();
        Self::set_keyboard_shortcuts();
    }

    pub(crate) fn input() -> Option<HtmlInputElement> {
        element("#search input#search-box").and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
    }

    pub(crate) fn set_keyboard_shortcuts() {
        let input = match Self::input() {
            None => return,
            Some(el) => el,
        };

        let search_focus_shortcut: Closure<dyn Fn(_)> = Closure::wrap(Box::new(move |e: Event| {
            match keyboard_event(&e) {
                Some(F_KEY) if !element_is_active(&input) => Self::focus(),
                Some(ESCAPE_KEY) if element_is_active(&input) => input.blur().expect("blurred"),
                _ => return,
            };

            e.prevent_default();
        }));

        window().set_onkeydown(Some(search_focus_shortcut.as_ref().unchecked_ref()));
        search_focus_shortcut.forget();
    }

    pub(crate) fn search_query() -> Option<String> {
        match Self::input() {
            Some(ref el) if !el.value().is_empty() => Some(el.value()),
            _ => None,
        }
    }

    pub(crate) fn focus() {
        if let Some(el) = Self::input() {
            el.focus().expect("focus succeeded");
            el.select();
        }
    }

    fn search_pipelines() -> impl Future<Item = (), Error = ()> {
        let query = Self::search_query().try_into().ok();

        Pipelines::fetch(query).and_then(|pipelines| {
            PipelinesView::update(&pipelines);
            futures::future::ok(())
        })
    }
}
