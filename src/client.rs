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

use std::str::FromStr;
use std::time::Duration;

use chrono::offset::Utc;
use chrono::DateTime;
use futures::compat::Future01CompatExt;
use futures_stable::Stream;
use http::Uri;
use hyper::client::HttpConnector;
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde_json;
use url::Url;

use crate::messages::ApiResult;
use crate::{Error, Result};

// TODO: query_timeout function
// TODO: use ToStr where possible

// TODO: replace Step with plain duration once it supports f64 output
// ref: https://github.com/rust-lang/rust/issues/54361
pub enum Step {
    Seconds(f64),
    Duration(Duration),
}

pub type HyperHttpsConnector = HttpsConnector<HttpConnector>;

// FIXME: why am I exposing the underlying connection type?
pub struct PromClient<T: hyper::client::connect::Connect + 'static> {
    client: Client<T, Body>,
    host: Url,
    query_timeout: Option<Duration>,
}

impl PromClient<HyperHttpsConnector> {
    pub fn new_https(
        host: &str,
        query_timeout: Option<Duration>,
    ) -> Result<PromClient<HyperHttpsConnector>> {
        let host = Url::from_str(host).map_err(|e| Error::new_invalid_host_error(host, e))?;
        // Explicitly unwrapping here because this library is unusable if you can't build an HTTPS connection pool
        let https = HttpsConnector::new(4).expect("Cannot build HTTPS connection pool");
        Ok(PromClient {
            client: Client::builder().keep_alive(true).build(https),
            host,
            query_timeout,
        })
    }
}

impl<T: hyper::client::connect::Connect + 'static> PromClient<T> {
    pub async fn instant_query(
        &mut self,
        query: String,
        at: Option<DateTime<Utc>>,
    ) -> Result<ApiResult> {
        // interesting: when there were problems with the await macro it flagged the wrong line
        let mut u = self.api_call_base_url("/api/v1/query");
        u.query_pairs_mut().append_pair("query", &query);
        if let Some(t) = at {
            u.query_pairs_mut()
                .append_pair("time", t.to_rfc3339().as_str());
        }
        if let Some(t) = self.query_timeout {
            u.query_pairs_mut()
                .append_pair("timeout", &t.as_secs().to_string());
        }
        let u = Uri::from_str(u.as_str())?;

        await!(self.make_http_get_api_call(u))
    }

    pub async fn range_query(
        &mut self,
        query: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: Step,
    ) -> Result<ApiResult> {
        let mut u = self.api_call_base_url("/api/v1/query_range");
        u.query_pairs_mut().append_pair("query", &query);
        u.query_pairs_mut()
            .append_pair("start", &start.to_rfc3339().to_string());
        u.query_pairs_mut()
            .append_pair("end", &end.to_rfc3339().to_string());
        let step: String = match step {
            Step::Seconds(f) => f.to_string(),
            Step::Duration(d) => format!("{}s", d.as_secs().to_string()),
        };
        u.query_pairs_mut().append_pair("step", &step);
        if let Some(t) = self.query_timeout {
            u.query_pairs_mut()
                .append_pair("timeout", &t.as_secs().to_string());
        }
        let u = Uri::from_str(u.as_str())?;

        await!(self.make_http_get_api_call(u))
    }

    pub async fn series(
        &mut self,
        selectors: Vec<String>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ApiResult> {
        let mut u = self.api_call_base_url("/api/v1/series");
        for s in selectors {
            u.query_pairs_mut().append_pair("match[]", &s);
        }
        u.query_pairs_mut()
            .append_pair("start", &start.to_rfc3339().to_string());
        u.query_pairs_mut()
            .append_pair("end", &end.to_rfc3339().to_string());
        if let Some(t) = self.query_timeout {
            u.query_pairs_mut()
                .append_pair("timeout", &t.as_secs().to_string());
        }
        let u = Uri::from_str(u.as_str())?;

        await!(self.make_http_get_api_call(u))
    }

    pub async fn label_names(&mut self) -> Result<ApiResult> {
        let u = self.api_call_base_url("/api/v1/labels");
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_http_get_api_call(u))
    }

    pub async fn label_values(&mut self, label_name: String) -> Result<ApiResult> {
        let u = self.api_call_base_url(&format!("/api/v1/{}/values", label_name));
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_http_get_api_call(u))
    }

    pub async fn targets(&mut self) -> Result<ApiResult> {
        let u = self.api_call_base_url("/api/v1/targets");
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_http_get_api_call(u))
    }

    pub async fn alert_managers(&mut self) -> Result<ApiResult> {
        let u = self.api_call_base_url("/api/v1/alertmanagers");
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_http_get_api_call(u))
    }

    pub async fn config(&mut self) -> Result<ApiResult> {
        let u = self.api_call_base_url("/api/v1/status/config");
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_http_get_api_call(u))
    }

    pub async fn flags(&mut self) -> Result<ApiResult> {
        let u = self.api_call_base_url("/api/v1/status/flags");
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_http_get_api_call(u))
    }

    async fn make_http_get_api_call(&mut self, u: Uri) -> Result<ApiResult> {
        let resp = await!(self.client.get(u).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<ApiResult>(&body).map_err(From::from)
    }

    //
    // TSDB Admin APIs
    //

    pub async fn delete_series(
        &mut self,
        series: Vec<String>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<ApiResult> {
        let mut u = self.api_call_base_url("/api/v1/admin/tsdb/delete_series");
        for s in series {
            u.query_pairs_mut().append_pair("match[]", &s);
        }
        if let Some(start) = start {
            u.query_pairs_mut()
                .append_pair("start", &start.to_rfc3339().to_string());
        }
        if let Some(end) = end {
            u.query_pairs_mut()
                .append_pair("end", &end.to_rfc3339().to_string());
        }
        if let Some(t) = self.query_timeout {
            u.query_pairs_mut()
                .append_pair("timeout", &t.as_secs().to_string());
        }
        let u = Uri::from_str(u.as_str())?;

        // Explicitly unwrapping here because this shouldn't fail,
        // and there's nothing a user can do if it does. this failure
        // is because of a library bug, not because of their input
        let post = Request::post(u)
            .body(Body::empty())
            .expect("Failed to construct 'delete_series' POST with an empty body");

        let resp = await!(self.client.request(post).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<ApiResult>(&body).map_err(From::from)
    }

    pub async fn snapshot(&mut self, skip_head: bool) -> Result<ApiResult> {
        let mut u = self.api_call_base_url("/api/v1/admin/tsdb/snapshot");
        u.query_pairs_mut().append_pair("skip_head", &skip_head.to_string());
        let u = Uri::from_str(u.as_str())?;

        // Explicitly unwrapping here because this shouldn't fail,
        // and there's nothing a user can do if it does. this failure
        // is because of a library bug, not because of their input
        let post = Request::post(u)
            .body(Body::empty())
            .expect("Failed to construct 'snapshot' POST with empty body");

        let resp = await!(self.client.request(post).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<ApiResult>(&body).map_err(From::from)
    }

    pub async fn clean_tombstones(&mut self) -> Result<ApiResult> {
        let u = self.api_call_base_url("/api/v1/admin/tsdb/clean_tombstones");
        let u = Uri::from_str(u.as_str())?;

        // Explicitly unwrapping here because this shouldn't fail,
        // and there's nothing a user can do if it does. this failure
        // is because of a library bug, not because of their input
        let post = Request::post(u)
            .body(Body::empty())
            .expect("Failed to construct 'clean_tombstones' POST with empty body");

        let resp = await!(self.client.request(post).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<ApiResult>(&body).map_err(From::from)
    }

    fn api_call_base_url(&self, api_path: &str) -> Url {
        // Explicitly unwrapping here because we should be able
        // to join an already-verified Prometheus hostname with
        // precanned valid path fragments
        self.host
            .clone()
            .join(api_path)
            .expect(&format!("Cannot create API url with path '{}'", api_path))
    }
}
