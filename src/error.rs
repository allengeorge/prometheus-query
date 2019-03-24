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
use std::{
    fmt,
    fmt::{Display, Formatter},
};

use http;
use http::uri;
use hyper;
use native_tls;
use serde_json;
use url;

// FIXME: use a proper return type
#[derive(Debug)]
pub struct Error {
    inner: Option<Box<dyn std::error::Error>>,
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.inner.as_ref().map(|b| b.as_ref())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), fmt::Error> {
        match self.inner.as_ref() {
            Some(e) => e.fmt(f),
            None => f.write_str("Unknown error"),
        }
    }
}

impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Error {
            inner: Some(Box::new(e)),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error {
            inner: Some(Box::new(e)),
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error {
            inner: Some(Box::new(e)),
        }
    }
}

impl From<uri::InvalidUri> for Error {
    fn from(e: uri::InvalidUri) -> Self {
        Error {
            inner: Some(Box::new(e)),
        }
    }
}

impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Self {
        Error {
            inner: Some(Box::new(e)),
        }
    }
}

impl From<http::Error> for Error {
    fn from(e: http::Error) -> Self {
        Error {
            inner: Some(Box::new(e)),
        }
    }
}
