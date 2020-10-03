use chrono::prelude::*;

pub struct Share{
    pub code: String,
    pub price: String,
    pub price_date: DateTime<Utc>,
}

impl Share{
    pub fn display_date(&self)->String{
        self.price_date.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}