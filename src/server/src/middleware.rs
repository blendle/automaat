use actix_service::{Service, Transform};
use actix_web::http::header;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::{ok, FutureResult};
use futures::{Future, Poll};

#[derive(Copy, Clone, Debug)]
pub(crate) struct RemoveContentLengthHeader;

impl<S, B> Transform<S> for RemoveContentLengthHeader
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RemoveContentLengthHeaderMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RemoveContentLengthHeaderMiddleware { service })
    }
}

pub(crate) struct RemoveContentLengthHeaderMiddleware<S> {
    service: S,
}

impl<S, B> Service for RemoveContentLengthHeaderMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, request: ServiceRequest) -> Self::Future {
        Box::new(self.service.call(request).and_then(|mut response| {
            response.headers_mut().remove(header::CONTENT_LENGTH);
            Ok(response)
        }))
    }
}
