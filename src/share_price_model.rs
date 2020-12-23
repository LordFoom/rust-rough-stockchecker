use chrono::prelude::*;
use std::collections::HashMap;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::string::ToString;
use strum_macros::Display;

pub struct Share{
    pub company_code: String,
    pub price: String,
    pub price_date: NaiveDateTime,
}

const DATE_FMT:  &str = "%Y-%m-%d \n%H:%M:%S";

impl Share{
    pub fn display_date(&self)->String{
        self.price_date.format(DATE_FMT).to_string()
    }
    pub fn pretty_price(&self) -> String {
        str::replace(self.price.trim(), ",", ".")
    }
    pub fn price_as_decimal(&self) -> Decimal {
        Decimal::from_str(&self.pretty_price()).unwrap_or_default()
    }
}


#[derive(Debug, PartialEq, Eq, Hash, Display)]
pub enum ShareMoment {
    Yesterday,
    LastWeek,
    LastMonth,
    LastYear,
}

pub struct ShareTimeline {
    pub share: Share,
    pub share_history:HashMap<ShareMoment, Share>
}

impl ShareTimeline{}