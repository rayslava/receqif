use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fmt;

pub struct Purchase {
    sum: i64,
    date: DateTime<Utc>,
    pub items: Vec<Item>,
}

impl Purchase {
    pub fn total_sum(&self) -> i64 {
        self.sum
    }

    pub fn date(&self) -> DateTime<Utc> {
        self.date
    }
}

#[derive(Deserialize, Debug, Clone)]
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
#[derive(Deserialize, Debug, Clone)]
struct Receipt {
    totalSum: i64,
    #[serde(with = "custom_date_format")]
    dateTime: DateTime<Utc>,
    pub items: Vec<Item>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Document {
    receipt: Receipt,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Ticket {
    document: Document,
}

mod custom_date_format {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{self, Deserialize, Deserializer};

    /// The format seems alike to RFC3339 but is not compliant
    const FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

    /// Custom deserializer for format in our json
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let naive_dt =
            NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
        // Associate the NaiveDateTime with the Utc timezone
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc))
    }
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Input {
    ticket: Ticket,
}

pub fn parse_purchase(line: &str) -> Purchase {
    // TODO: Check if several receipts are possible
    let receipt: Vec<Input> = serde_json::from_str(line).unwrap();
    Purchase {
        sum: receipt[0].ticket.document.receipt.totalSum,
        date: receipt[0].ticket.document.receipt.dateTime,
        items: receipt[0].ticket.document.receipt.items.clone(),
    }
}

#[cfg(test)]
mod receipttest {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn item() {
        let line = String::from(
            r#"
{
    "quantity" : 1,
    "nds" : 1,
    "price" : 5549,
    "paymentType": 4,
    "productType": 1,
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
    "dateTime": "2021-03-24T17:42:00",
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
    ]
}
"#,
        );

        let testit: Receipt = serde_json::from_str(&line).unwrap();
        assert_eq!(testit.totalSum, 548702);
        assert_eq!(testit.items.len(), 2);
        assert_eq!(testit.items[0].sum, 5549);
        assert_eq!(testit.items[1].sum, 20599);
        assert_eq!(testit.dateTime.day(), 24);
        assert_eq!(testit.dateTime.month(), 3);
        assert_eq!(testit.dateTime.year(), 2021);
    }

    #[test]
    fn input() {
        let line = String::from(
            r#"
{
"ticket": {
  "document" : {
    "receipt" : {
      "totalSum" : 548702,
      "userInn" : "7703270067",
      "operator" : "Теpминал 24",
      "dateTime": "2021-03-24T17:42:00",
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
      ]
    }
  }
}
}
"#,
        );

        let testit: Input = serde_json::from_str(&line).unwrap();
        assert_eq!(testit.ticket.document.receipt.totalSum, 548702);
        assert_eq!(testit.ticket.document.receipt.items.len(), 2);
        assert_eq!(testit.ticket.document.receipt.items[0].sum, 5549);
        assert_eq!(testit.ticket.document.receipt.items[1].sum, 20599);
        assert_eq!(testit.ticket.document.receipt.dateTime.day(), 24);
        assert_eq!(testit.ticket.document.receipt.dateTime.month(), 3);
        assert_eq!(testit.ticket.document.receipt.dateTime.year(), 2021);
    }

    #[test]
    fn display() {
        let it = Item {
            name: "test".to_string(),
            sum: 1000,
        };
        let line = it.to_string();
        assert_eq!(line, "test:1000");
    }
}
