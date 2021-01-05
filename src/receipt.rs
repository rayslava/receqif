use serde::Deserialize;
use std::fmt;

#[derive(Deserialize)]
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
#[derive(Deserialize)]
struct Receipt {
    totalSum: i64,
    items: Vec<Item>,
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

pub fn parse_receipt(line: &str) -> Vec<Item> {
    let result: Input = serde_json::from_str(&line).unwrap();
    result.document.receipt.items
}

#[cfg(test)]
mod receipt {
    use super::*;

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
    ]
}
"#,
        );

        let testit: Receipt = serde_json::from_str(&line).unwrap();
        assert_eq!(testit.totalSum, 548702);
        assert_eq!(testit.items.len(), 2);
        assert_eq!(testit.items[0].sum, 5549);
        assert_eq!(testit.items[1].sum, 20599);
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
      ]
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
