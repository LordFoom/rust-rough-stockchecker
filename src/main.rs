#[macro_use]
extern crate prettytable;

use std::borrow::Borrow;
use std::str::FromStr;

use chrono::prelude::*;
use clap::{App, Arg, ArgMatches};
use mysql::{OptsBuilder, params, Pool, PooledConn};
use mysql::prelude::*;
use prettytable::{Attr, Cell, color, Row, Table};
use regex::Regex;
use rust_decimal::Decimal;
use select::document::Document;
use select::predicate::Name;

use crate::share_price_model::{Share, ShareTimeline, ShareHistoryPoint};
use std::collections::HashMap;

mod share_price_model;
mod config_options;
mod db_model;

fn init() -> ArgMatches {
    App::new("Share price checker")
        .version("1.0")
        .author("Foom <lordfoom@gmail.com>")
        .about("Scrape price changes from Google")
        .arg(Arg::new("code")
                 .value_name("COMPANY_CODE")
                 .index(1)
                 .required(true)
                 .multiple(true)
             // .validator(is_valid_code)
             ,
        )
        .get_matches()
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = init();
    let company_codes: Vec<_> = args.values_of("code").unwrap().collect();

    let mut company_prices = get_company_prices(company_codes).await?;
    print_prices(&company_prices);
    if let Err(e) = save_prices(company_prices) {
        panic!("Couldn't save prices {}", e.to_string())
    }


    Ok(())
}

/**
Will return a vector of a map of a company
*/
// #[tokio::main]
async fn get_company_prices(company_codes: Vec<&str>) -> Result<Vec<ShareTimeline>, Box<dyn std::error::Error>> {
    let mut company_prices = Vec::new();
    for company_code in company_codes {
        let res = reqwest::get(&format!("https://www.google.com/search?hl=en&q=share+price+{}", company_code)).await?;
        let body = res.text().await?;
        // println!("Body:\n{}", body);
        let search_doc = Document::from(body.borrow());
        // let spans = search_doc.find(Name("span")).collect();
        let starts_with_digits = Regex::new(r"(^[\d+\s]*\d+,.\d+\s)").unwrap();

        let mut price = String::new();

        for span in search_doc.find(Name("div")) {
            let txt = span.text();
            if txt.contains("Currency in ")
                && starts_with_digits.is_match(&txt){
                let captures = starts_with_digits.captures(&txt).unwrap();
                price = str::replace(&captures[1], ",", ".");
                // println!("div=={}",  txt);
                break;
            }
        }
        //create share object from whence we just loaded
        let now = Utc::now().naive_utc();
        let company_curr = Share {
            code: company_code.to_string(),
            price: price.clone(),
            price_date: now,
        };

        //we should save the above share here?
        //NO! But we also shouldn't load history below


        //todo, we need to move this into its own method and out of here
        let mut share_history = HashMap::new();

        if let Some(share) = load_share_history(company_code, 1){
            share_history.insert(ShareHistoryPoint::Yesterday, share);
        }
        if let Some(share) = load_share_history(company_code, 7){
            share_history.insert(ShareHistoryPoint::LastWeek, share);
        }
        if let Some(share) = load_share_history(company_code,30){
            share_history.insert(ShareHistoryPoint::LastMonth, share);
        }


        let share_timeline = ShareTimeline {
            share: company_curr,
            share_history
        };
        // let share_comparison = ShareComparison {
        //     share: company,
        //     share_history: company_history,
        // };
        // println!("{} = {} at {}",
        //          company.code,
        //          company.price,
        //          company.price_date.format("%Y-%m-%d %H:%M:%S").to_string()
        // );

        company_prices.push(share_timeline);
    }
    Ok(company_prices)
}

fn load_share_history(company_code:&str, days_ago_upper_limit: i32)->Option<Share>{
    let mut conn = get_db_connection().unwrap();
    let base_select = r"SELECT company_code as code, price, price_date
                         FROM stock_prices WHERE company_code = :code
                         AND date(price_date) <= curdate() ";
    let suffix_select:String = String::from("- INTERVAL {} DAY ORDER BY id DESC");
    let yest_select = format!("{} - INTERVAL {} DAY ORDER BY id DESC ", base_select, days_ago_upper_limit);
    match conn.exec_first(yest_select,
                            params! {"code"=>company_code}) {
        Ok(Some((code, price, price_date))) => Some(Share { code, price, price_date }),
        Ok(None) => None,
        Err(e) => panic!("Unable to get previous company info: {}", e.to_string()),
    }
}

fn print_prices(company_prices: &Vec<ShareTimeline>) {
    let mut tbl = Table::new();
    tbl.add_row(Row::new(vec![
        Cell::new("CODE")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BLUE))
        ,
        Cell::new("CURRENT PRICE")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::YELLOW))
        ,
        Cell::new("CURR TIME")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::YELLOW))
        ,
        Cell::new("LAST PRICE")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_BLUE))
        ,
        Cell::new("LAST PRICE TIME")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_BLUE))
        ,
        Cell::new("MOVEMENT")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_YELLOW))
        ,
        Cell::new("%")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BRIGHT_YELLOW))
        ,
    ]));
    for share_timeline in company_prices {

        let str_curr_price = share_timeline.share.pretty_price();
        let str_yest_price = price_str_at_moment(share_timeline, ShareHistoryPoint::Yesterday);
        let str_last_week_price = price_str_at_moment(share_timeline, ShareHistoryPoint::LastWeek);
        let str_last_month_price = price_str_at_moment(share_timeline, ShareHistoryPoint::LastMonth);
        let now_price = match Decimal::from_str(&str_curr_price) {
            Ok(dec) => dec,
            Err(e) => panic!("Couldn't make price into a number: {}", e.to_string())
        };
        let last_price = match Decimal::from_str(&str_yest_price) {
            Ok(dec) => dec,
            Err(e) => panic!("Couldn't make price into a number: {}", e.to_string())
        };

        let movement = now_price - last_price;
        let percentage = (now_price / last_price) * Decimal::from_str("100.00").unwrap()-Decimal::from(100);
        // let mut percen
        // let mut percentage_string = format!("{}%", percentage.to_string());
        let (movement_style, percentage_string) = if movement < Decimal::from_str("0.0").unwrap() {
            (Attr::ForegroundColor(color::RED), format!("-{:.5}%", percentage))
        } else {
            (Attr::ForegroundColor(color::GREEN), format!("+{:.5}%", percentage))
        };

        tbl.add_row(Row::new(vec![
            Cell::new(&share_timeline.code()),
            Cell::new(&share_timeline.price()),
            Cell::new(&share_timeline.latest_date()),
            Cell::new(&share_timeline.historic_price()),
            Cell::new(&share_timeline.historic_date()),
            Cell::new(&movement.to_string())
                .with_style(Attr::Bold)
                .with_style(movement_style),
            Cell::new(&percentage_string)
                .with_style(Attr::Bold)
                .with_style(movement_style),
        ]));
    }

    tbl.printstd();
}

fn price_str_at_moment(share_timeline: ShareTimeLine, moment: ShareHistoryPoint)->String {
     match share_timeline.get_share_at_moment(moment){
        Some(share) => share.pretty_price(),
        _ => String::from("---")
    }
}

//Save the  current prices
fn save_prices(company_prices: Vec<ShareTimeline>) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = get_db_connection()?;
    //db connection
    // let mut conn = get_db_connection();

    //create table if needed
    conn.query_drop(
        r"CREATE TABLE IF NOT EXISTS stock_prices
                 ( id bigint auto_increment,
                   company_code varchar(255),
                   price decimal(10,2),
                   price_date datetime,
                   primary key(id)
                 );
                   "
    )?;
    //insert into table
    conn.exec_batch(
        r"INSERT INTO stock_prices(company_code, price, price_date)
                VALUES (:code, :price, now())",
        company_prices
            .iter()
            .map(|company_time_line| params! {
                        "code" => &company_time_line.share.code,
                        "price" => str::replace(&company_time_line.share.price, ",", "."),
                    }
            ))?;

    Ok(())
}

fn get_db_connection() -> Result<PooledConn, Box<dyn std::error::Error>> {
    let conn_details = match config_options::read_db_config() {

        Ok(connection) => connection,
        Err(e) => panic!("TIME TO DIE, CONNECTION! {}", e.to_string()),
    };
    //read db connection stuff from database
    let builder = OptsBuilder::new()
        .user(Some(conn_details.username))
        .pass(Some(conn_details.password))
        .db_name(Some(conn_details.database));

    let pool = Pool::new(builder)?;
    Ok(pool.get_conn()?)
}