//! A session represents the authenticated state of a user.

use dodrio::{RootRender, VdomWeak};
use futures::future::Future;

/// The actions a controller has to implement to bridge between the UI and the
/// model.
pub(crate) trait Actions {
    /// Try to authenticate with the server using the provided token.
    ///
    /// If the authentication succeeds, this method should return to the last
    /// visited page. If it fails, it should activate the login component.
    fn authenticate(
        root: &mut dyn RootRender,
        vdom: VdomWeak,
        token: String,
    ) -> Box<dyn Future<Item = (), Error = ()>>;
}
