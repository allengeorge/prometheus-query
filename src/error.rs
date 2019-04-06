// Copyright 2019 Allen A. George
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::convert::From;
use std::error::Error as StdError;
use std::result::Result as StdResult;
use std::{
    fmt,
    fmt::{Display, Formatter},
};

use http;
use http::uri;
use hyper;
use serde_json;
use url;

/// Type alias for `Result<T, prometheus_query::Error>`
pub type Result<T> = std::result::Result<T, Error>;

/// Represents errors that can occur while making queries to Prometheus.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

/// High-level error categories. New categories may be added later.
#[derive(Debug)]
pub enum ErrorKind {
    /// Invalid Prometheus host URL.
    InvalidHost {
        /// String that could not be parsed into a valid URL.
        url: String,
        /// Underlying error type.
        err: url::ParseError,
    },
    /// Invalid Prometheus API call URL.
    /// This _could_ happen because of arguments that cannot be
    /// encoded into the URL path fragment or query parameters.
    InvalidApiUrl {
        /// String that could not be parsed into a valid URL.
        url: String,
        /// Underlying error type.
        err: uri::InvalidUri,
    },
    /// General HTTP client error.
    /// Triggered when making HTTP requests to, or reading responses from Prometheus.
    Http {
        /// Underlying error type.
        err: hyper::Error,
    },
    /// API response JSON-parsing error.
    /// Triggered when the library cannot parse the API response from Prometheus.
    InvalidResponseJson {
        /// TODO: include the full API url for which this error is occurring
        /// Underlying error type.
        err: serde_json::Error,
    },
    /// Destructuring should not be exhaustive.
    ///
    /// This enum may grow additional variants, so this makes sure clients
    /// won't break as additional variants are added.
    #[doc(hidden)]
    __Nonexhaustive,
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn StdError> {
        match self.kind {
            ErrorKind::InvalidHost { ref err, .. } => Some(err),
            ErrorKind::Http { ref err } => Some(err),
            ErrorKind::InvalidResponseJson { ref err, .. } => Some(err),
            _ => unreachable!("unexpected match arm!"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> StdResult<(), fmt::Error> {
        match self.kind {
            ErrorKind::InvalidHost { ref url, .. } => {
                f.write_str(&format!("Invalid Prometheus host '{}'", url))
            }
            ErrorKind::Http { ref err } => err.fmt(f),
            ErrorKind::InvalidApiUrl { ref url, .. } => {
                f.write_str(&format!("Invalid API url '{}'", url))
            }
            ErrorKind::InvalidResponseJson { ref err, .. } => err.fmt(f),
            _ => unreachable!("unexpected match arm!"),
        }
    }
}

// TODO: include new_ functions for all error types
impl Error {
    pub(crate) fn new_invalid_host_error<S: Into<String>>(url: S, err: url::ParseError) -> Error {
        Error {
            kind: ErrorKind::InvalidHost {
                url: url.into(),
                err,
            },
        }
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error {
            kind: ErrorKind::Http { err },
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error {
            kind: ErrorKind::InvalidResponseJson { err },
        }
    }
}

impl From<uri::InvalidUri> for Error {
    fn from(err: uri::InvalidUri) -> Self {
        Error {
            kind: ErrorKind::InvalidApiUrl {
                url: err.to_string(),
                err,
            },
        }
    }
}
