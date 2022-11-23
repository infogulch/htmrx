#![allow(dead_code)]

use rocket::{
    request::{FromRequest, Outcome},
    Request,
};

// TODO: typed header values
// TODO: handle multi-value headers

pub struct Header<'a, const NAME: &'static str> {
    pub value: &'a str,
}

#[rocket::async_trait]
impl<'r, const NAME: &'static str> FromRequest<'r> for Header<'r, NAME> {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.headers().get_one(NAME) {
            Some(r) => Outcome::Success(Header { value: r }),
            None => Outcome::Forward(()),
        }
    }
}

pub type IfMatch<'a> = Header<'a, "If-Match">;
pub type HXRequest<'a> = Header<'a, "HX-Request">;
