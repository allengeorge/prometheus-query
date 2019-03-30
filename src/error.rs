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

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    InvalidHost {
        url: String,
        err: url::ParseError,
    },
    InvalidApiUrl {
        url: String,
        err: uri::InvalidUri,
    },
    Http {
        err: hyper::Error,
    },
    CannotParseResponseJson {
        // FIXME: add API call name
        err: serde_json::Error,
    },
    Other(String),

    /// Destructing should not be exhaustive.
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
            ErrorKind::CannotParseResponseJson { ref err, .. } => Some(err),
            _ => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> StdResult<(), fmt::Error> {
        match self.kind {
            ErrorKind::InvalidHost { ref url, .. } => {
                f.write_str(&format!("Invalid Prometheus host '{}'", url)) // FIXME: print err
            },
            ErrorKind::Http { ref err } => err.fmt(f),
            ErrorKind::InvalidApiUrl { ref url, ref err } =>  {
                f.write_str(&format!("Cannot build url '{}'", url)) // FIXME: print err
            },
            ErrorKind::CannotParseResponseJson { ref err, .. } => err.fmt(f),
            ErrorKind::Other(ref s) => f.write_str(&s),
            _ => unreachable!("unexpected match arm!"),
        }
    }
}

pub(crate) fn new_invalid_host_error<S: Into<String>>(url: S, err: url::ParseError) -> Error {
    Error {
        kind: ErrorKind::InvalidHost {
            url: url.into(),
            err,
        },
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
            kind: ErrorKind::CannotParseResponseJson { err },
        }
    }
}

impl From<uri::InvalidUri> for Error {
    fn from(err: uri::InvalidUri) -> Self {
        Error {
            kind: ErrorKind::InvalidApiUrl { url: err.to_string(), err },
        }
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error {
            kind: ErrorKind::Other(s.to_owned()),
        }
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error {
            kind: ErrorKind::Other(s),
        }
    }
}
