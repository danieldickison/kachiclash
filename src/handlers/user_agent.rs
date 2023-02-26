use result::ResultOptionExt;
use std::fmt::Display;

use actix_web::error::ParseError;
use actix_web::http::header::{
    Header, HeaderName, HeaderValue, InvalidHeaderValue, TryIntoHeaderValue, USER_AGENT,
};

pub struct UserAgent(String);

impl Default for UserAgent {
    fn default() -> Self {
        "unknown user agent".into()
    }
}

impl From<&str> for UserAgent {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl Display for UserAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryIntoHeaderValue for UserAgent {
    type Error = InvalidHeaderValue;

    fn try_into_value(self) -> Result<HeaderValue, Self::Error> {
        HeaderValue::from_str(&self.0)
    }
}

impl Header for UserAgent {
    fn name() -> HeaderName {
        USER_AGENT
    }

    fn parse<M: actix_web::HttpMessage>(msg: &M) -> Result<Self, ParseError> {
        Ok(msg
            .headers()
            .get(Self::name())
            .map(|h| h.to_str().map(Self::from).map_err(|_| ParseError::Header))
            .invert()?
            .unwrap_or_default())
    }
}
