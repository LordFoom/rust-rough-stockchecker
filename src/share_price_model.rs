use chrono::prelude::*;
use std::collections::HashMap;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::string::ToString;
use rust_decimal::prelude::Zero;
use strum_macros::Display;
use crate::share_price_model;

pub struct Share{
    pub code: String,
    pub price: String,
    pub price_date: NaiveDateTime,
}

const DATE_FMT:  &str = "%Y-%m-%d %H:%M:%S";

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
    Current,
    Yesterday,
    DaysAgo(i32),
    LastWeek,
    LastMonth,
    LastYear,
}

pub struct ShareTimeline {
    pub share: Share,
    pub share_history:HashMap<ShareMoment, Share>
}

impl ShareTimeline {
    pub fn get_share_at_moment(&self, share_hst_pnt: &share_price_model::ShareMoment) -> Option<&Share> {
        self.share_history.get(share_hst_pnt)
    }

    pub fn get_price_at_moment_or_zero(&self, share_hst_pnt: &share_price_model::ShareMoment) -> Decimal {
       match self.share_history.get(share_hst_pnt) {
           Some(share) => share.price_as_decimal(),
           _ => Decimal::zero(),
       }
    }
    pub fn get_pretty_price_at_moment_or_filler(&self, moment: &ShareMoment) -> String {
        match self.share_history.get(moment){
            Some(hist_share) => hist_share.price.to_string(),
            _ => String::from("---"),
        }
    }
}

pub struct ShareComparison{
    pub share: Share,
    pub share_history: Option<Share>
}

impl ShareComparison{
    pub fn code(&self) -> String {
        self.share.code.to_string()
    }

    pub fn price(&self)->String {
        self.share.price.to_string()
    }

    pub fn latest_date(&self)->String{
        self.share.display_date()
    }

    pub fn historic_price(&self)->String{
        match &self.share_history{
            Some(hist_share) => hist_share.price.to_string(),
            _ => String::from("---")
        }
    }

    pub fn historic_date(&self)->String{
        match &self.share_history{
            Some(hist_share) => hist_share.display_date(),
            _ => String::from("---")
        }
    }

}