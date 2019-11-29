// TODO: unused right now, but committed to keep track of.

use dodrio::{Node, Render, RenderContext};
use std::marker::PhantomData;

pub(crate) struct TaskMenu<C> {
    active: bool,
    _controller: PhantomData<C>,
}

impl<C> TaskMenu<C> {
    pub(crate) const fn new(active: bool) -> Self {
        Self {
            active,
            _controller: PhantomData,
        }
    }
}

trait RenderParts<'b> {
    /// The trigger/button to open/close the menu
    fn trigger(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The main menu wrapping the contents.
    fn menu(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The menu item to mark the task as a favourite.
    fn item_favourite(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The menu item to mark the task as a "one-click" task.
    fn item_one_click(&self, cx: &mut RenderContext<'b>) -> Node<'b>;

    /// The menu item to mark the task as a "one-click" task.
    fn item_help(&self, cx: &mut RenderContext<'b>) -> Node<'b>;
}

impl<'b, C> RenderParts<'b> for TaskMenu<C> {
    fn trigger(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        div(&cx)
            .child(
                button(&cx)
                    .attr("type", "button")
                    .child(span(&cx).child(i(&cx).finish()).finish())
                    .child(span(&cx).child(text(" Menu")).finish())
                    .finish(),
            )
            .finish()
    }

    fn menu(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let items = [
            self.item_one_click(cx),
            hr(&cx).finish(),
            self.item_favourite(cx),
            hr(&cx).finish(),
            self.item_help(cx),
        ];

        div(&cx)
            .attr("role", "menu")
            .attr("class", "dropdown-menu")
            .child(div(&cx).children(items).finish())
            .finish()
    }

    fn item_favourite(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let link = a(&cx).child(i(&cx).finish()).child(text(" Favourite"));
        let txt = "You can set this pipeline as one of your favorites. \
                   It will show up at the top of your search results.";
        let description = div(&cx).child(p(&cx).child(text(txt)).finish());

        div(&cx)
            .attr("class", "favourite")
            .children([link.finish(), description.finish()])
            .finish()
    }

    fn item_one_click(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let link = a(&cx).child(i(&cx).finish()).child(text(" One-Click"));
        let txt = "First provide all the variables, then save your preset \
                   as a one-click task for easy future use.";
        let description = div(&cx).child(p(&cx).child(text(txt)).finish());

        div(&cx)
            .attr("class", "one-click")
            .children([link.finish(), description.finish()])
            .finish()
    }

    fn item_help(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let link = a(&cx).child(i(&cx).finish()).child(text(" Ask Help"));
        let txt = "Need any help? Ask us anything you need! ";
        let description = div(&cx).child(p(&cx).child(text(txt)).child(i(&cx).finish()).finish());

        div(&cx)
            .attr("class", "help")
            .children([link.finish(), description.finish()])
            .finish()
    }
}

impl<C> Render for TaskMenu<C> {
    fn render<'b>(&self, cx: &mut RenderContext<'b>) -> Node<'b> {
        use dodrio::builder::*;

        let class = if self.active {
            "menu is-active"
        } else {
            "menu"
        };

        div(&cx)
            .attr("class", class)
            .children([self.trigger(cx), self.menu(cx)])
            .finish()
    }
}
