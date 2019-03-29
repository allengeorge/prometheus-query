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
use crate::Result;

// TODO: query_timeout function

// TODO: replace Step with plain duration
pub enum Step {
    Seconds(f64),
    Duration(Duration),
}

pub type HyperHttpsConnector = HttpsConnector<HttpConnector>;

pub struct PromClient<T: hyper::client::connect::Connect + 'static> {
    client: Client<T, Body>,
    hostname: Url,
    query_timeout: Option<Duration>,
}

impl PromClient<HyperHttpsConnector> {
    pub fn new_https(
        hostname: &str,
        query_timeout: Option<Duration>,
    ) -> Result<PromClient<HyperHttpsConnector>> {
        let hostname = Url::from_str(hostname)?;
        let https = HttpsConnector::new(4)?;
        Ok(PromClient {
            client: Client::builder().keep_alive(true).build(https),
            hostname,
            query_timeout,
        })
    }
}

impl<T: hyper::client::connect::Connect + 'static> PromClient<T> {
    pub async fn instant_query(
        &mut self,
        query: String, // FIXME: turn into &str
        at: Option<DateTime<Utc>>,
    ) -> Result<ApiResult> {
        // interesting: when there were problems with the await macro it flagged the wrong line
        let u = self.instant_query_uri(&query, at)?;
        await!(self.make_prometheus_api_call(u))
    }

    fn instant_query_uri(&self, query: &str, at: Option<DateTime<Utc>>) -> Result<Uri> {
        let mut u = self.hostname.clone().join("/api/v1/query")?;
        {
            let mut serializer = u.query_pairs_mut();
            serializer.append_pair("query", query);
            at.map(|t| serializer.append_pair("time", t.to_rfc3339().as_str()));
            self.query_timeout
                .map(|d| serializer.append_pair("timeout", &d.as_secs().to_string()));
        }
        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn range_query(
        &mut self,
        query: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: Step,
    ) -> Result<ApiResult> {
        let u = self.range_query_uri(query, start, end, step)?;
        await!(self.make_prometheus_api_call(u))
    }

    fn range_query_uri(
        &self,
        query: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: Step,
    ) -> Result<Uri> {
        let mut u = self.hostname.clone().join("/api/v1/query_range")?;

        {
            let mut serializer = u.query_pairs_mut();

            serializer.append_pair("query", &query);

            let start = start.to_rfc3339().to_string();
            serializer.append_pair("start", &start);

            let end = end.to_rfc3339().to_string();
            serializer.append_pair("end", &end);

            let step: String = match step {
                Step::Seconds(f) => f.to_string(),
                Step::Duration(d) => format!("{}s", d.as_secs().to_string()),
            };
            serializer.append_pair("step", &step);

            if let Some(t) = self.query_timeout {
                serializer.append_pair("timeout", &t.as_secs().to_string());
            }
        }

        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn series(
        &mut self,
        selectors: Vec<String>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ApiResult> {
        let u = self.series_uri(selectors, start, end)?;
        await!(self.make_prometheus_api_call(u))
    }

    fn series_uri(
        &self,
        selectors: Vec<String>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Uri> {
        let mut u = self.hostname.clone().join("/api/v1/series")?;

        {
            let mut serializer = u.query_pairs_mut();

            for s in selectors {
                serializer.append_pair("match[]", &s);
            }

            let start = start.to_rfc3339().to_string();
            serializer.append_pair("start", &start);

            let end = end.to_rfc3339().to_string();
            serializer.append_pair("end", &end);

            if let Some(t) = self.query_timeout {
                serializer.append_pair("timeout", &t.as_secs().to_string());
            }
        }

        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn label_names(&mut self) -> Result<ApiResult> {
        let u = self.hostname.clone().join("/api/v1/labels")?;
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_prometheus_api_call(u))
    }

    pub async fn label_values(&mut self, label_name: String) -> Result<ApiResult> {
        let path = format!("/api/v1/{}/values", label_name);
        let u = self.hostname.clone().join(&path)?;
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_prometheus_api_call(u))
    }

    async fn make_prometheus_api_call(&mut self, u: Uri) -> Result<ApiResult> {
        let resp = await!(self.client.get(u).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<ApiResult>(&body).map_err(From::from)
    }

    pub async fn targets(&mut self) -> Result<ApiResult> {
        let u = self.hostname.clone().join("/api/v1/targets")?;
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_prometheus_api_call(u))
    }

    pub async fn alert_managers(&mut self) -> Result<ApiResult> {
        let u = self.hostname.clone().join("/api/v1/alertmanagers")?;
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_prometheus_api_call(u))
    }

    pub async fn flags(&mut self) -> Result<ApiResult> {
        let u = self.hostname.clone().join("/api/v1/flags")?;
        let u = Uri::from_str(u.as_str())?;
        await!(self.make_prometheus_api_call(u))
    }

    pub async fn delete_series(
        &mut self,
        series: Vec<String>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<ApiResult> {
        let u = self.delete_series_uri(series, start, end)?;

        let post = Request::post(u).body(Body::empty())?;
        let resp = await!(self.client.request(post).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<ApiResult>(&body).map_err(From::from)
    }

    fn delete_series_uri(
        &self,
        series: Vec<String>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<Uri> {
        let mut u = self
            .hostname
            .clone()
            .join("/api/v1/admin/tsdb/delete_series")?;

        {
            let mut serializer = u.query_pairs_mut();

            for s in series {
                serializer.append_pair("match[]", &s);
            }

            if let Some(start) = start {
                let start = start.to_rfc3339().to_string();
                serializer.append_pair("start", &start);
            }

            if let Some(end) = end {
                let end = end.to_rfc3339().to_string();
                serializer.append_pair("end", &end);
            }

            if let Some(t) = self.query_timeout {
                serializer.append_pair("timeout", &t.as_secs().to_string());
            }
        }

        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn clean_tombstones(&mut self) -> Result<ApiResult> {
        let u = self
            .hostname
            .clone()
            .join("/api/v1/admin/tsdb/clean_tombstones")?;
        let u = Uri::from_str(u.as_str())?;

        let post = Request::post(u).body(Body::empty())?;

        let resp = await!(self.client.request(post).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<ApiResult>(&body).map_err(From::from)
    }
}

//fn config() -> impl Future {}
//fn snapshot() -> impl Future {}
