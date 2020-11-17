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

use crate::share_price_model::{Share, ShareComparison};

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
    print_prices(&mut company_prices);
    if let Err(e) = save_prices(&mut company_prices) {
        panic!("Couldn't save prices {}", e.to_string())
    }


    Ok(())
}

// #[tokio::main]
async fn get_company_prices(company_codes: Vec<&str>) -> Result<Vec<ShareComparison>, Box<dyn std::error::Error>> {
    let mut conn = get_db_connection()?;
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
                && starts_with_digits.is_match(&txt)
            {
                let captures = starts_with_digits.captures(&txt).unwrap();
                price = str::replace(&captures[1], ",", ".");
                // println!("div=={}",  txt);
                break;
            }
        }
        //get the previous price from the company, if it exists
        let company_history = match conn.exec_first(r"SELECT company_code as code, price, price_date
                         FROM stock_prices WHERE company_code = :code
                         AND date(price_date) <= curdate() - INTERVAL 1 DAY ORDER BY id DESC",
                                                    params! {"code"=>company_code.to_string()}) {
            Ok(Some((code, price, price_date))) => Some(Share { code, price, price_date }),
            Ok(None) => None,
            Err(e) => panic!("Unable to get previous company info: {}", e.to_string()),
        };

        let now = Utc::now().naive_utc();
        let company = Share {
            code: company_code.to_string(),
            price: price.clone(),
            price_date: now,
        };

        let share_comparison = ShareComparison {
            share: company,
            share_history: company_history,
        };
        // println!("{} = {} at {}",
        //          company.code,
        //          company.price,
        //          company.price_date.format("%Y-%m-%d %H:%M:%S").to_string()
        // );

        company_prices.push(share_comparison);
    }
    Ok(company_prices)
}

fn print_prices(company_prices: &mut Vec<ShareComparison>) {
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
    for share_comparison in company_prices {
        let proper_decimal = str::replace(&share_comparison.price().trim(), ",", ".");
        let proper_historic_decimal = str::replace(&share_comparison.historic_price().trim(), ",", ".");
        let now_price = match Decimal::from_str(&proper_decimal) {
            Ok(dec) => dec,
            Err(e) => panic!("Couldn't make price into a number: {}", e.to_string())
        };
        let last_price = match Decimal::from_str(&proper_historic_decimal) {
            Ok(dec) => dec,
            Err(e) => panic!("Couldn't make price into a number: {}", e.to_string())
        };

        let movement = now_price - last_price;
        let percentage = (((now_price / last_price) * Decimal::from_str("100.00").unwrap())-Decimal::from(100));
        // let mut percen
        let mut percentage_string = format!("{}%", percentage.to_string());
        let (movement_style, percentage_string) = if movement < Decimal::from_str("0.0").unwrap() {
            (Attr::ForegroundColor(color::RED), format!("-{:.5}%", percentage))
        } else {
            (Attr::ForegroundColor(color::GREEN), format!("+{:.5}%", percentage))
        };

        tbl.add_row(Row::new(vec![
            Cell::new(&share_comparison.code()),
            Cell::new(&share_comparison.price()),
            Cell::new(&share_comparison.latest_date()),
            Cell::new(&share_comparison.historic_price()),
            Cell::new(&share_comparison.historic_date()),
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

fn save_prices(company_prices: &mut Vec<ShareComparison>) -> Result<(), Box<dyn std::error::Error>> {
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
            .map(|company| params! {
                        "code" => &company.share.code,
                        "price" => str::replace(&company.share.price, ",", "."),
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