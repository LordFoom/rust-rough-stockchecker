use chrono::prelude::*;

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