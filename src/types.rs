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

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use serde::{
    de,
    de::{MapAccess, SeqAccess, Unexpected, Visitor},
    ser::{SerializeStruct, SerializeTuple},
    {Deserialize, Deserializer, Serialize, Serializer},
};
use url::Url;
use url_serde::{De, Ser};

const PROM_INFINITY: &str = "Inf";

const PROM_NEGATIVE_INFINITY: &str = "-Inf";

const PROM_NAN: &str = "NaN";

pub enum Step {
    Seconds(f64),
    Duration(Duration),
}

// FIXME: use FromStr to ToStr

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum QueryResult {
    Success(QuerySuccess),
    Error(QueryError),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QuerySuccess {
    // FIXME:
    #[serde(default)]
    pub data: Option<Data>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QueryError {
    #[serde(rename = "errorType")]
    pub error_type: String,
    #[serde(rename = "error")]
    pub error_message: String,
    #[serde(default)]
    pub data: Option<Data>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl Error for QueryError {}

impl Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.error_message)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Data {
    Expression(Expression),
    Series(Vec<Metric>),
    LabelsOrValues(Vec<String>),
    Targets(Targets),
    AlertManagers(AlertManagers),
    Flags(HashMap<String, String>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "resultType", content = "result")]
pub enum Expression {
    #[serde(rename = "scalar")]
    Scalar(Sample),
    #[serde(rename = "string")]
    String(StringSample),
    #[serde(rename = "vector")]
    Instant(Vec<Instant>),
    #[serde(rename = "matrix")]
    Range(Vec<Range>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Instant {
    pub metric: Metric,
    #[serde(rename = "value")]
    pub sample: Sample,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Range {
    pub metric: Metric,
    #[serde(rename = "values")]
    pub samples: Vec<Sample>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Metric {
    #[serde(flatten)]
    pub labels: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub epoch: f64,
    pub value: f64,
}

impl<'de> Deserialize<'de> for Sample {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitorImpl;

        impl<'de> Visitor<'de> for VisitorImpl {
            type Value = Sample;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Prometheus sample")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let epoch = seq
                    .next_element::<f64>()?
                    .ok_or_else(|| de::Error::missing_field("sample time"))?;
                let value = seq
                    .next_element::<&str>()?
                    .ok_or_else(|| de::Error::missing_field("sample value"))?;

                let value = match value {
                    PROM_INFINITY => std::f64::INFINITY,
                    PROM_NEGATIVE_INFINITY => std::f64::NEG_INFINITY,
                    PROM_NAN => std::f64::NAN,
                    _ => value
                        .parse::<f64>()
                        .map_err(|_| de::Error::invalid_value(Unexpected::Str(value), &self))?,
                };

                Ok(Sample { epoch, value })
            }
        }

        deserializer.deserialize_seq(VisitorImpl)
    }
}

impl Serialize for Sample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_tuple(2)?;
        s.serialize_element(&self.epoch)?;
        s.serialize_element(&self.value)?;
        s.end()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringSample {
    pub epoch: f64,
    pub value: String,
}

impl<'de> Deserialize<'de> for StringSample {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitorImpl;

        impl<'de> Visitor<'de> for VisitorImpl {
            type Value = StringSample;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Prometheus string sample")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let epoch = seq
                    .next_element::<f64>()?
                    .ok_or_else(|| de::Error::missing_field("sample time"))?;
                let value = seq
                    .next_element::<String>()?
                    .ok_or_else(|| de::Error::missing_field("sample value"))?;

                Ok(StringSample { epoch, value })
            }
        }

        deserializer.deserialize_seq(VisitorImpl)
    }
}

impl Serialize for StringSample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_tuple(2)?;
        s.serialize_element(&self.epoch)?;
        s.serialize_element(&self.value)?;
        s.end()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Targets {
    #[serde(default, rename = "activeTargets")]
    pub active: Vec<ActiveTarget>,
    #[serde(default, rename = "droppedTargets")]
    pub dropped: Vec<DroppedTarget>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTarget {
    pub discovered_labels: HashMap<String, String>,
    pub labels: HashMap<String, String>,
    #[serde(with = "url_serde")]
    pub scrape_url: Url,
    #[serde(
        deserialize_with = "empty_string_is_none",
        serialize_with = "none_to_empty_string"
    )]
    pub last_error: Option<String>,
    #[serde(
        deserialize_with = "rfc3339_to_date_time",
        serialize_with = "date_time_to_rfc3339"
    )]
    pub last_scrape: DateTime<FixedOffset>,
    #[serde(
        deserialize_with = "deserialize_health",
        serialize_with = "serialize_health"
    )]
    pub health: TargetHealth,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TargetHealth {
    Up,
    Down,
    Unknown,
}

fn empty_string_is_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let o: Option<String> = Option::deserialize(d)?;
    Ok(o.filter(|s| !s.is_empty()))
}

fn none_to_empty_string<S: Serializer>(
    s: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if let Some(v) = s {
        serializer.serialize_str(&v)
    } else {
        serializer.serialize_str("")
    }
}

fn rfc3339_to_date_time<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<DateTime<FixedOffset>, D::Error> {
    let s = String::deserialize(d)?;
    DateTime::from_str(&s).map_err(de::Error::custom)
}

fn date_time_to_rfc3339<S: Serializer>(
    v: &DateTime<FixedOffset>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&v.to_rfc3339())
}

fn deserialize_health<'de, D: Deserializer<'de>>(d: D) -> Result<TargetHealth, D::Error> {
    let o: Option<String> = Option::deserialize(d)?;
    Ok(o.map_or(TargetHealth::Unknown, |s| match s.as_str() {
        "up" => TargetHealth::Up,
        "down" => TargetHealth::Down,
        _ => TargetHealth::Unknown,
    }))
}

fn serialize_health<S: Serializer>(
    health: &TargetHealth,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let value = match health {
        TargetHealth::Up => "up",
        TargetHealth::Down => "down",
        TargetHealth::Unknown => "unknown",
    };

    serializer.serialize_str(value)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedTarget {
    pub discovered_labels: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AlertManagers {
    #[serde(default, rename = "activeAlertmanagers")]
    pub active: Vec<AlertManager>,
    #[serde(default, rename = "droppedAlertmanagers")]
    pub dropped: Vec<AlertManager>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlertManager {
    pub url: Url,
}

impl<'de> Deserialize<'de> for AlertManager {
    fn deserialize<D>(deserializer: D) -> Result<AlertManager, D::Error>
    where
        D: Deserializer<'de>,
    {
        // variant of: https://serde.rs/deserialize-struct.html

        struct VisitorImpl;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Url,
        };

        const FIELDS: &[&str] = &["url"];

        impl<'de> Visitor<'de> for VisitorImpl {
            type Value = AlertManager;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("AlertManager")
            }

            fn visit_map<V>(self, mut map: V) -> Result<AlertManager, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut url: Option<Url> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Url => {
                            if url.is_some() {
                                return Err(de::Error::duplicate_field("url"));
                            }
                            url = De::into_inner(map.next_value()?); // FIXME: how does this work??!
                        }
                    }
                }
                let url = url.ok_or_else(|| de::Error::missing_field("url"))?;
                Ok(AlertManager { url })
            }
        }

        deserializer.deserialize_struct("AlertManager", &FIELDS, VisitorImpl)
    }
}

impl Serialize for AlertManager {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("AlertManager", 1)?;
        s.serialize_field("url", &Ser::new(&self.url))?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{DateTime, FixedOffset};
    use url::Url;

    use crate::types::{
        ActiveTarget, AlertManager, AlertManagers, Data, DroppedTarget, Expression, Instant,
        Metric, QueryError, QueryResult, QuerySuccess, Range, Sample, StringSample, TargetHealth,
        Targets,
    };

    #[test]
    fn should_deserialize_json_error() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status": "error",
            "error": "Major",
            "errorType": "Seriously Bad"
        }
        "#;

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Error(QueryError {
                error_message: "Major".to_string(),
                error_type: "Seriously Bad".to_string(),
                data: None,
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_error_with_instant_and_warnings() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status": "error",
            "error": "This is a strange error",
            "errorType": "Weird",
            "warnings": [
                "You timed out, foo"
            ],
            "data" : {
                "resultType" : "vector",

                "result" : [
                    {
                        "metric" : {
                            "__name__" : "up",
                            "job" : "prometheus",
                            "instance" : "localhost:9090"
                        },
                        "value": [ 1435781451.781, "1" ]
                    },
                    {
                        "metric" : {
                            "__name__" : "up",
                            "job" : "node",
                            "instance" : "localhost:9100"
                        },
                        "value" : [ 1435781451.781, "0" ]
                    }
                ]
            }
        }
        "#;

        let mut metric_1: HashMap<String, String> = HashMap::new();
        metric_1.insert("__name__".to_owned(), "up".to_owned());
        metric_1.insert("job".to_owned(), "prometheus".to_owned());
        metric_1.insert("instance".to_owned(), "localhost:9090".to_owned());

        let mut metric_2: HashMap<String, String> = HashMap::new();
        metric_2.insert("__name__".to_owned(), "up".to_owned());
        metric_2.insert("job".to_owned(), "node".to_owned());
        metric_2.insert("instance".to_owned(), "localhost:9100".to_owned());

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Error(QueryError {
                error_type: "Weird".to_owned(),
                error_message: "This is a strange error".to_owned(),
                data: Some(Data::Expression(Expression::Instant(vec!(
                    Instant {
                        metric: Metric {
                            labels: metric_1.clone(),
                        },
                        sample: Sample {
                            epoch: 1435781451.781,
                            value: 1 as f64,
                        },
                    },
                    Instant {
                        metric: Metric {
                            labels: metric_2.clone(),
                        },
                        sample: Sample {
                            epoch: 1435781451.781,
                            value: 0 as f64,
                        },
                    },
                )))),
                warnings: vec!["You timed out, foo".to_owned()],
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_scalar() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status": "success",
            "data": {
                "resultType": "scalar",
                "result": [1435781451.781, "1"]
            }
        }
        "#;

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Some(Data::Expression(Expression::Scalar(Sample {
                    epoch: 1435781451.781,
                    value: 1 as f64,
                }))),
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_scalar_with_warnings() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "warnings": ["You timed out, foo"],
            "status": "success",
            "data": {
                "resultType": "scalar",
                "result": [1435781451.781, "1"]
            }
        }
        "#;

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Some(Data::Expression(Expression::Scalar(Sample {
                    epoch: 1435781451.781,
                    value: 1 as f64,
                }))),
                warnings: vec!["You timed out, foo".to_owned()],
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_string() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status": "success",
            "data": {
                "resultType": "string",
                "result": [1435781451.781, "foo"]
            }
        }
        "#;

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Some(Data::Expression(Expression::String(StringSample {
                    epoch: 1435781451.781,
                    value: "foo".to_owned(),
                }))),
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_vector() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status" : "success",
            "data" : {
                "resultType" : "vector",
                "result" : [
                    {
                        "metric" : {
                            "__name__" : "up",
                            "job" : "prometheus",
                            "instance" : "localhost:9090"
                        },
                        "value": [ 1435781451.781, "1" ]
                    },
                    {
                        "metric" : {
                            "__name__" : "up",
                            "job" : "node",
                            "instance" : "localhost:9100"
                        },
                        "value" : [ 1435781451.781, "0" ]
                    }
                ]
            }
        }
        "#;

        let mut metric_1: HashMap<String, String> = HashMap::new();
        metric_1.insert("__name__".to_owned(), "up".to_owned());
        metric_1.insert("job".to_owned(), "prometheus".to_owned());
        metric_1.insert("instance".to_owned(), "localhost:9090".to_owned());

        let mut metric_2: HashMap<String, String> = HashMap::new();
        metric_2.insert("__name__".to_owned(), "up".to_owned());
        metric_2.insert("job".to_owned(), "node".to_owned());
        metric_2.insert("instance".to_owned(), "localhost:9100".to_owned());

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Some(Data::Expression(Expression::Instant(vec!(
                    Instant {
                        metric: Metric {
                            labels: metric_1.clone(),
                        },
                        sample: Sample {
                            epoch: 1435781451.781,
                            value: 1 as f64,
                        },
                    },
                    Instant {
                        metric: Metric {
                            labels: metric_2.clone(),
                        },
                        sample: Sample {
                            epoch: 1435781451.781,
                            value: 0 as f64,
                        },
                    },
                )))),
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_matrix() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status" : "success",
            "data" : {
                "resultType" : "matrix",
                "result" : [
                    {
                        "metric" : {
                            "__name__" : "up",
                            "job" : "prometheus",
                            "instance" : "localhost:9090"
                        },
                        "values" : [
                           [ 1435781430.781, "1" ],
                           [ 1435781445.781, "1" ],
                           [ 1435781460.781, "1" ]
                        ]
                    },
                    {
                        "metric" : {
                            "__name__" : "up",
                            "job" : "node",
                            "instance" : "localhost:9091"
                        },
                        "values" : [
                           [ 1435781430.781, "0" ],
                           [ 1435781445.781, "0" ],
                           [ 1435781460.781, "1" ]
                        ]
                    }
                ]
            }
        }
        "#;

        let mut metric_1: HashMap<String, String> = HashMap::new();
        metric_1.insert("__name__".to_owned(), "up".to_owned());
        metric_1.insert("job".to_owned(), "prometheus".to_owned());
        metric_1.insert("instance".to_owned(), "localhost:9090".to_owned());

        let mut metric_2: HashMap<String, String> = HashMap::new();
        metric_2.insert("__name__".to_owned(), "up".to_owned());
        metric_2.insert("job".to_owned(), "node".to_owned());
        metric_2.insert("instance".to_owned(), "localhost:9091".to_owned());

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Some(Data::Expression(Expression::Range(vec!(
                    Range {
                        metric: Metric {
                            labels: metric_1.clone(),
                        },
                        samples: vec!(
                            Sample {
                                epoch: 1435781430.781,
                                value: 1 as f64,
                            },
                            Sample {
                                epoch: 1435781445.781,
                                value: 1 as f64,
                            },
                            Sample {
                                epoch: 1435781460.781,
                                value: 1 as f64,
                            },
                        ),
                    },
                    Range {
                        metric: Metric {
                            labels: metric_2.clone(),
                        },
                        samples: vec!(
                            Sample {
                                epoch: 1435781430.781,
                                value: 0 as f64,
                            },
                            Sample {
                                epoch: 1435781445.781,
                                value: 0 as f64,
                            },
                            Sample {
                                epoch: 1435781460.781,
                                value: 1 as f64,
                            },
                        ),
                    },
                )))),
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_labels() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status" : "success",
            "data" :[
                "__name__",
                "call",
                "code",
                "config",
                "dialer_name",
                "endpoint",
                "event",
                "goversion",
                "handler",
                "instance",
                "interval",
                "job",
                "le",
                "listener_name",
                "name",
                "quantile",
                "reason",
                "role",
                "scrape_job",
                "slice",
                "version"
            ]
        }
        "#;

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Some(Data::LabelsOrValues(vec![
                    "__name__".to_owned(),
                    "call".to_owned(),
                    "code".to_owned(),
                    "config".to_owned(),
                    "dialer_name".to_owned(),
                    "endpoint".to_owned(),
                    "event".to_owned(),
                    "goversion".to_owned(),
                    "handler".to_owned(),
                    "instance".to_owned(),
                    "interval".to_owned(),
                    "job".to_owned(),
                    "le".to_owned(),
                    "listener_name".to_owned(),
                    "name".to_owned(),
                    "quantile".to_owned(),
                    "reason".to_owned(),
                    "role".to_owned(),
                    "scrape_job".to_owned(),
                    "slice".to_owned(),
                    "version".to_owned(),
                ],)),
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_label_values() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status" : "success",
            "data" :[
                "node",
                "prometheus"
            ]
        }
        "#;

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Some(Data::LabelsOrValues(vec![
                    "node".to_owned(),
                    "prometheus".to_owned(),
                ],)),
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_targets() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status": "success",
            "data": {
                "activeTargets": [
                    {
                        "discoveredLabels": {
                            "__address__": "127.0.0.1:9090",
                            "__metrics_path__": "/metrics",
                            "__scheme__": "http",
                            "job": "prometheus"
                        },
                        "labels": {
                            "instance": "127.0.0.1:9090",
                            "job": "prometheus"
                        },
                        "scrapeUrl": "http://127.0.0.1:9090/metrics",
                        "lastError": "",
                        "lastScrape": "2017-01-17T15:07:44.723715405+01:00",
                        "health": "up"
                    }
                ],
                "droppedTargets": [
                    {
                        "discoveredLabels": {
                            "__address__": "127.0.0.1:9100",
                            "__metrics_path__": "/metrics",
                            "__scheme__": "http",
                            "job": "node"
                        }
                    }
                ]
            }
        }
        "#;

        let mut active_discovered_labels: HashMap<String, String> = HashMap::new();
        active_discovered_labels.insert("__address__".to_owned(), "127.0.0.1:9090".to_owned());
        active_discovered_labels.insert("__metrics_path__".to_owned(), "/metrics".to_owned());
        active_discovered_labels.insert("__scheme__".to_owned(), "http".to_owned());
        active_discovered_labels.insert("job".to_owned(), "prometheus".to_owned());
        let active_discovered_labels = active_discovered_labels;

        let mut active_labels: HashMap<String, String> = HashMap::new();
        active_labels.insert("instance".to_owned(), "127.0.0.1:9090".to_owned());
        active_labels.insert("job".to_owned(), "prometheus".to_owned());
        let active_labels = active_labels;

        let mut dropped_discovered_labels: HashMap<String, String> = HashMap::new();
        dropped_discovered_labels.insert("__address__".to_owned(), "127.0.0.1:9100".to_owned());
        dropped_discovered_labels.insert("__metrics_path__".to_owned(), "/metrics".to_owned());
        dropped_discovered_labels.insert("__scheme__".to_owned(), "http".to_owned());
        dropped_discovered_labels.insert("job".to_owned(), "node".to_owned());
        let dropped_discovered_labels = dropped_discovered_labels;

        let last_scrape: DateTime<FixedOffset> =
            DateTime::parse_from_rfc3339("2017-01-17T15:07:44.723715405+01:00").unwrap();

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            res,
            QueryResult::Success(QuerySuccess {
                data: Some(Data::Targets(Targets {
                    active: vec![ActiveTarget {
                        discovered_labels: active_discovered_labels,
                        labels: active_labels,
                        scrape_url: Url::parse("http://127.0.0.1:9090/metrics").unwrap(),
                        last_error: None,
                        last_scrape,
                        health: TargetHealth::Up
                    },],
                    dropped: vec![DroppedTarget {
                        discovered_labels: dropped_discovered_labels
                    },],
                })),
                warnings: Vec::new(),
            })
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_alert_managers() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status": "success",
            "data": {
                "activeAlertmanagers": [
                    {
                        "url": "http://127.0.0.1:9090/api/v1/alerts"
                    }
                ],
                "droppedAlertmanagers": [
                    {
                        "url": "http://127.0.0.1:9093/api/v1/alerts"
                    }
                ]
            }
        }
        "#;

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Some(Data::AlertManagers(AlertManagers {
                    active: vec![AlertManager {
                        url: Url::parse("http://127.0.0.1:9090/api/v1/alerts").unwrap(),
                    },],
                    dropped: vec![AlertManager {
                        url: Url::parse("http://127.0.0.1:9093/api/v1/alerts").unwrap(),
                    },],
                })),
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }

    #[test]
    fn should_deserialize_json_prom_flags() -> Result<(), std::io::Error> {
        let j = r#"
        {
            "status": "success",
            "data": {
                "alertmanager.notification-queue-capacity": "10000",
                "alertmanager.timeout": "10s",
                "log.level": "info",
                "query.lookback-delta": "5m",
                "query.max-concurrency": "20"
            }
        }
        "#;

        let mut flags: HashMap<String, String> = HashMap::new();
        flags.insert(
            "alertmanager.notification-queue-capacity".to_owned(),
            "10000".to_owned(),
        );
        flags.insert("alertmanager.timeout".to_owned(), "10s".to_owned());
        flags.insert("log.level".to_owned(), "info".to_owned());
        flags.insert("query.lookback-delta".to_owned(), "5m".to_owned());
        flags.insert("query.max-concurrency".to_owned(), "20".to_owned());
        let flags = flags;

        let res = serde_json::from_str::<QueryResult>(j)?;
        assert_eq!(
            QueryResult::Success(QuerySuccess {
                data: Data::Flags(flags),
                warnings: Vec::new(),
            }),
            res
        );

        Ok(())
    }
}
