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

//! Async interface to the [Prometheus HTTP V1 API](https://prometheus.io/docs/prometheus/latest/querying/api/).
//! This crate allows you to query a Prometheus server to drive tools
//! like CLIs etc. It uses **nightly-only** `async`/`await`, and `futures`.

#![feature(custom_attribute)]
#![feature(futures_api, async_await, await_macro)]

pub use client::{PromClient, Step};
pub use error::{Error, Result};

mod client;
mod error;
pub mod messages;

// FIXME: remove need to have 'to_owned()' everywhere
