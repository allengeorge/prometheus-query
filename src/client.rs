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

use crate::messages::QueryResult;
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
        let uri = self.instant_query_url(&query, at, query_timeout)?;
        let resp = await!(self.client.get(uri).compat())?;
        let body = await!(resp.into_body().concat2().compat())?;
        serde_json::from_slice::<QueryResult>(&body).map_err(From::from)
    }

    fn instant_query_url(
        &mut self,
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
}

//fn range_query() -> impl Future {
//    unimplemented!()
//}
//
//fn series() -> impl Future {}
//
//fn label_names() -> impl Future {}
//
//fn label_values() -> impl Future {}
//
//fn targets() -> impl Future {}
//
//fn rules() -> impl Future {}
//
//fn alerts() -> impl Future {}
//
//fn config() -> impl Future {}
//
//fn flags() -> impl Future {}
//
//fn snapshot() -> impl Future {}
//
//fn delete_series() -> impl Future {}
//
//fn clean_tombstones() -> impl Future {}
