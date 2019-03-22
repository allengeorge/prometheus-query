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
use hyper::Body;
use hyper::Client;
use hyper_tls::HttpsConnector;
use serde_json;
use url::Url;

use crate::types::{QueryResult, Step};
use crate::{Error, Result};

pub type HyperHttpsConnector = HttpsConnector<HttpConnector>;

pub struct PromClient<T: hyper::client::connect::Connect + 'static> {
    client: Client<T, Body>,
    url: Url,
}

impl PromClient<HyperHttpsConnector> {
    pub fn new_https(
        endpoint: &str,
    ) -> std::result::Result<PromClient<HyperHttpsConnector>, Error> {
        let url = Url::from_str(endpoint)?;
        let https = HttpsConnector::new(4)?;
        Ok(PromClient {
            client: Client::builder().keep_alive(true).build(https),
            url,
        })
    }
}

impl<T: hyper::client::connect::Connect + 'static> PromClient<T> {
    pub async fn instant_query(
        &mut self,
        query: String, // FIXME: turn into &str
        at: Option<DateTime<Utc>>,
        query_timeout: Option<Duration>,
    ) -> Result {
        // interesting: when there were problems with the await macro it flagged the wrong line
        let u = self.instant_query_uri(&query, at, query_timeout)?;
        await!(self.make_prometheus_api_call(u))
    }

    fn instant_query_uri(
        &self,
        query: &str,
        at: Option<DateTime<Utc>>,
        query_timeout: Option<Duration>,
    ) -> std::result::Result<Uri, Error> {
        let mut u = self.url.clone().join("/api/v1/query")?;
        {
            let mut serializer = u.query_pairs_mut();
            serializer.append_pair("query", query);
            at.map(|t| serializer.append_pair("time", t.to_rfc3339().as_str()));
            query_timeout.map(|d| serializer.append_pair("timeout", &d.as_secs().to_string()));
        }
        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn range_query(
        &mut self,
        query: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: Step,
        timeout: Option<Duration>,
    ) -> Result {
        let u = self.range_query_uri(query, start, end, step, timeout)?;
        await!(self.make_prometheus_api_call(u))
    }

    fn range_query_uri(
        &self,
        query: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: Step,
        timeout: Option<Duration>,
    ) -> std::result::Result<Uri, Error> {
        let mut u = self.url.clone().join("/api/v1/query_range")?;

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

            if let Some(t) = timeout {
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
    ) -> Result {
        let u = self.series_uri(selectors, start, end)?;
        await!(self.make_prometheus_api_call(u))
    }

    fn series_uri(
        &self,
        selectors: Vec<String>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> std::result::Result<Uri, Error> {
        let mut u = self.url.clone().join("/api/v1/series")?;

        {
            let mut serializer = u.query_pairs_mut();

            for s in selectors {
                serializer.append_pair("match[]", &s);
            }

            let start = start.to_rfc3339().to_string();
            serializer.append_pair("start", &start);

            let end = end.to_rfc3339().to_string();
            serializer.append_pair("end", &end);
        }

        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn label_names(&mut self) -> Result {
        let u = self.label_names_uri()?;
        await!(self.make_prometheus_api_call(u))
    }

    fn label_names_uri(&self) -> std::result::Result<Uri, Error> {
        let u = self.url.clone().join("/api/v1/labels")?;
        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn label_values(&mut self, label_name: String) -> Result {
        let u = self.label_values_uri(label_name)?;
        await!(self.make_prometheus_api_call(u))
    }

    fn label_values_uri(&self, label_name: String) -> std::result::Result<Uri, Error> {
        let path = format!("/api/v1/{}/values", label_name);
        let u = self.url.clone().join(&path)?;
        Uri::from_str(u.as_str()).map_err(From::from)
    }

    async fn make_prometheus_api_call(&mut self, u: Uri) -> Result {
        let resp = await!(self.client.get(u).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<QueryResult>(&body).map_err(From::from)
    }

    pub async fn targets(&mut self) -> Result {
        let u = self.targets_uri()?;
        await!(self.make_prometheus_api_call(u))
    }

    fn targets_uri(&self) -> std::result::Result<Uri, Error> {
        let u = self.url.clone().join("/api/v1/targets")?;
        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn alert_managers(&mut self) -> Result {
        let u = self.alert_managers_uri()?;
        await!(self.make_prometheus_api_call(u))
    }

    fn alert_managers_uri(&self) -> std::result::Result<Uri, Error> {
        let u = self.url.clone().join("/api/v1/alertmanagers")?;
        Uri::from_str(u.as_str()).map_err(From::from)
    }

    pub async fn flags(&mut self) -> Result {
        let u = self.flags_uri()?;
        await!(self.make_prometheus_api_call(u))
    }

    fn flags_uri(&self) -> std::result::Result<Uri, Error> {
        let u = self.url.clone().join("/api/v1/flags")?;
        Uri::from_str(u.as_str()).map_err(From::from)
    }
}

//fn config() -> impl Future {}
//fn snapshot() -> impl Future {}
//
//fn delete_series() -> impl Future {}
//
//fn clean_tombstones() -> impl Future {}
