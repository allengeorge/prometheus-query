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

#![feature(async_await, await_macro, futures_api)]

use std::error::Error as StdError;
use std::result::Result as StdResult;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use clap::{App, AppSettings, Arg, SubCommand};
use futures::{FutureExt, TryFutureExt};
use tokio;

use prometheus_query::{messages::ApiResult, PromClient};

// XXX: remember: if you accidentally return the wrong value from an async function
// the compiler compares everything to that _wrong_ return value as opposed to the type
// specified in the function signature

fn main() -> Result<(), std::io::Error> {
    let app = cli();
    let matches = app.get_matches();

    let hostname = matches.value_of("HOSTNAME").unwrap().to_owned();
    let query_timeout = matches.value_of("timeout");

    if let Some(matches) = matches.subcommand_matches("instant") {
        let query = matches.value_of("QUERY").unwrap().to_owned();
        let at = matches.value_of("at");
        tokio::run({
            instant_query(
                hostname,
                query,
                at.map(|v| v.to_owned()),
                query_timeout.map(|v| v.to_owned()),
            )
            .map(|r| {
                println!("{:#?}", &r);
                Ok(())
            })
            .boxed()
            .compat()
        });
    } else if let Some(matches) = matches.subcommand_matches("delete") {
        let series = matches
            .values_of("SERIES")
            .unwrap()
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();
        let start = matches.value_of("start");
        let end = matches.value_of("end");
        tokio::run({
            delete_series(
                hostname,
                series,
                start.map(|v| v.to_owned()),
                end.map(|v| v.to_owned()),
                query_timeout.map(|v| v.to_owned()),
            )
            .map(|r| {
                println!("{:#?}", &r);
                Ok(())
            })
            .boxed()
            .compat()
        });
    }

    Ok(())
}

fn cli<'a, 'b>() -> App<'a, 'b> {
    App::new("Prometheus Query Client")
        .version("0.1")
        .author("Allen George <allen.george@gmail.com>")
        .about("Prometheus Query Client")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("HOSTNAME")
                .help("Prometheus hostname")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("timeout")
                .help("Query timeout")
                .short("o")
                .long("timeout")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("instant")
                .about("Instant query")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("QUERY")
                        .help("Query string")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("at")
                        .help("Instant at which the query should be evaluated")
                        .short("a")
                        .long("at")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("delete")
                .about("Delete series")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("SERIES")
                        .help("Series")
                        .required(true)
                        .min_values(1)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("start")
                        .help("Start time from which to delete series data")
                        .short("s")
                        .long("start")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("end")
                        .help("End time from to delete series data")
                        .short("e")
                        .long("end")
                        .takes_value(true),
                ),
        )
}

async fn instant_query(
    hostname: String,
    query: String,
    at: Option<String>,
    query_timeout: Option<String>,
) -> StdResult<ApiResult, Box<StdError + 'static>> {
    let at = date_time(at)?;
    let query_timeout = if let Some(v) = query_timeout {
        let v = v.parse::<u64>()?;
        Some(Duration::new(v, 0))
    } else {
        None
    };

    let mut p = PromClient::new_https(&hostname, query_timeout)?;
    let v = await!(p.instant_query(query, at));
    v.map_err(From::from)
}

async fn delete_series(
    hostname: String,
    series: Vec<String>,
    start: Option<String>,
    end: Option<String>,
    query_timeout: Option<String>,
) -> StdResult<ApiResult, Box<StdError + 'static>> {
    let start = date_time(start)?;
    let end = date_time(end)?;
    let query_timeout = if let Some(v) = query_timeout {
        let v = v.parse::<u64>()?;
        Some(Duration::new(v, 0))
    } else {
        None
    };

    let mut p = PromClient::new_https(&hostname, query_timeout)?;
    let v = await!(p.delete_series(series, start, end));
    v.map_err(From::from)
}

fn date_time(dt: Option<String>) -> StdResult<Option<DateTime<Utc>>, Box<StdError + 'static>> {
    if let Some(v) = dt {
        let v = v.parse::<i64>()?;
        let v = Utc.timestamp(v, 0);
        Ok(Some(v))
    } else {
        Ok(None)
    }
}
