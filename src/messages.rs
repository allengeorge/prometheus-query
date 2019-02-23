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

use serde::{
    de,
    de::{SeqAccess, Unexpected, Visitor},
    ser::SerializeTuple,
    {Deserialize, Deserializer, Serialize, Serializer},
};

const PROM_INFINITY: &str = "Inf";

const PROM_NEGATIVE_INFINITY: &str = "-Inf";

const PROM_NAN: &str = "NaN";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "status")]
pub enum QueryResult {
    Success(QuerySuccess),
    Error(QueryError),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct QuerySuccess {
    pub data: Data,
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
#[serde(tag = "resultType", content = "result")]
pub enum Data {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::messages::{
        Data, Instant, Metric, QueryError, QueryResult, QuerySuccess, Range, Sample, StringSample,
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
            res,
            QueryResult::Error(QueryError {
                error_message: "Major".to_string(),
                error_type: "Seriously Bad".to_string(),
                data: None,
                warnings: Vec::new(),
            })
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
            res,
            QueryResult::Error(QueryError {
                error_type: "Weird".to_owned(),
                error_message: "This is a strange error".to_owned(),
                data: Some(Data::Instant(vec!(
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
                ))),
                warnings: vec!["You timed out, foo".to_owned()],
            })
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
            res,
            QueryResult::Success(QuerySuccess {
                data: Data::Scalar(Sample {
                    epoch: 1435781451.781,
                    value: 1 as f64,
                }),
                warnings: Vec::new(),
            })
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
            res,
            QueryResult::Success(QuerySuccess {
                data: Data::Scalar(Sample {
                    epoch: 1435781451.781,
                    value: 1 as f64,
                }),
                warnings: vec!["You timed out, foo".to_owned()],
            })
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
            res,
            QueryResult::Success(QuerySuccess {
                data: Data::String(StringSample {
                    epoch: 1435781451.781,
                    value: "foo".to_owned(),
                }),
                warnings: Vec::new(),
            })
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
            res,
            QueryResult::Success(QuerySuccess {
                data: Data::Instant(vec!(
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
                )),
                warnings: Vec::new(),
            })
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
            res,
            QueryResult::Success(QuerySuccess {
                data: Data::Range(vec!(
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
                )),
                warnings: Vec::new(),
            })
        );

        Ok(())
    }
}
