use chrono::{Date, Utc};
use serde::Deserialize;
use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Item {
    pub name: String,
    pub sum: i64,
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.name, self.sum)
    }
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct Receipt {
    totalSum: i64,
    #[serde(with = "custom_date_format")]
    dateTime: Date<Utc>,
    pub items: Vec<Item>,
}

impl Receipt {
    pub fn total_sum(self) -> i64 {
        self.totalSum
    }

    pub fn date(&self) -> Date<Utc> {
        self.dateTime
    }
}

mod custom_date_format {
    use chrono::{Date, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer};

    /// The format seems alike to RFC3339 but is not compliant
    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

    /// Custom deserializer for format in our json
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Date<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = Utc
            .datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom);
        match dt {
            Ok(date) => Ok(date.date()),
            Err(e) => Err(e),
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Document {
    receipt: Receipt,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Input {
    document: Document,
}

pub fn parse_receipt(line: &str) -> Receipt {
    let result: Input = serde_json::from_str(&line).unwrap();
    result.document.receipt
}

#[cfg(test)]
mod receipt {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn item() {
        let line = String::from(
            r#"
{
    "quantity" : 1,
    "ndsRate" : 1,
    "price" : 5549,
    "calculationSubjectSign" : 1,
    "calculationTypeSign" : 4,
    "name" : "ХРЕН РУССКИЙ 170Г",
    "sum" : 5549
}
"#,
        );

        let testit: Item = serde_json::from_str(&line).unwrap();
        assert_eq!(testit.name, "ХРЕН РУССКИЙ 170Г");
        assert_eq!(testit.sum, 5549);
    }

    #[test]
    fn receipt() {
        let line = String::from(
            r#"
{
    "totalSum" : 548702,
    "userInn" : "7703270067",
    "operator" : "Теpминал 24",
    "items" : [
        {
	    "quantity" : 1,
	    "ndsRate" : 1,
	    "price" : 5549,
	    "calculationSubjectSign" : 1,
	    "calculationTypeSign" : 4,
	    "name" : "ХРЕН РУССКИЙ 170Г",
	    "sum" : 5549
        },
        {
	    "quantity" : 1,
	    "ndsRate" : 1,
	    "price" : 20599,
	    "calculationSubjectSign" : 1,
	    "calculationTypeSign" : 4,
	    "name" : "СОУС ОСТР.380Г КИНТО",
	    "sum" : 20599
        }
    ],
    "dateTime" : "2020-06-19T17:12:00"
}
"#,
        );

        let testit: Receipt = serde_json::from_str(&line).unwrap();
        assert_eq!(testit.totalSum, 548702);
        assert_eq!(testit.items.len(), 2);
        assert_eq!(testit.items[0].sum, 5549);
        assert_eq!(testit.items[1].sum, 20599);
        assert_eq!(testit.dateTime.day(), 19);
        assert_eq!(testit.dateTime.month(), 6);
        assert_eq!(testit.dateTime.year(), 2020);
    }

    #[test]
    fn input() {
        let line = String::from(
            r#"
{
  "document" : {
    "receipt" : {
      "totalSum" : 548702,
      "userInn" : "7703270067",
      "operator" : "Теpминал 24",
      "items" : [
        {
          "quantity" : 1,
          "ndsRate" : 1,
          "price" : 5549,
          "calculationSubjectSign" : 1,
          "calculationTypeSign" : 4,
          "name" : "ХРЕН РУССКИЙ 170Г",
          "sum" : 5549
        },
        {
          "quantity" : 1,
          "ndsRate" : 1,
          "price" : 20599,
          "calculationSubjectSign" : 1,
          "calculationTypeSign" : 4,
          "name" : "СОУС ОСТР.380Г КИНТО",
          "sum" : 20599
        }
      ],
    "dateTime" : "2020-06-19T17:12:00"
    }
  }
}
"#,
        );

        let testit: Input = serde_json::from_str(&line).unwrap();
        assert_eq!(testit.document.receipt.totalSum, 548702);
        assert_eq!(testit.document.receipt.items.len(), 2);
        assert_eq!(testit.document.receipt.items[0].sum, 5549);
        assert_eq!(testit.document.receipt.items[1].sum, 20599);
        assert_eq!(testit.document.receipt.dateTime.day(), 19);
        assert_eq!(testit.document.receipt.dateTime.month(), 6);
        assert_eq!(testit.document.receipt.dateTime.year(), 2020);
    }

    #[test]
    fn display() {
        let it = Item {
            name: "test".to_string(),
            sum: 1000,
        };
        let line = format!("{}", it.to_string());
        assert_eq!(line, "test:1000");
    }
}
