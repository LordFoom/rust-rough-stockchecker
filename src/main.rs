#[macro_use]
extern crate prettytable;

use std::borrow::Borrow;

use chrono::prelude::*;
use clap::{App, Arg, ArgMatches};
use mysql::{OptsBuilder, Pool, params};
use mysql::prelude::*;
use prettytable::Table;
use regex::Regex;
use select::document::Document;
use select::predicate::Name;

use crate::share_price_model::Share;

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
async fn main() -> Result<(), reqwest::Error> {
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
async fn get_company_prices(company_codes: Vec<&str>) -> Result<Vec<Share>, reqwest::Error> {
    let mut company_prices = Vec::new();
    for company_code in company_codes {
        let res = reqwest::get(&format!("https://www.google.com/search?hl=en&q={}", company_code)).await?;
        let body = res.text().await?;
        // println!("Body:\n{}", body);
        let search_doc = Document::from(body.borrow());
        // let spans = search_doc.find(Name("span")).collect();
        let starts_with_digits = Regex::new(r"(^[\d+\s]*\d+,\d+\s)").unwrap();

        let mut price = String::new();

        for span in search_doc.find(Name("div")) {
            let txt = span.text();
            if txt.contains("Currency in ")
                && starts_with_digits.is_match(&txt)
            {
                let captures = starts_with_digits.captures(&txt).unwrap();
                price = captures[1].to_string();
                // println!("div=={}",  txt);
                break;
            }
        }

        let now = Utc::now();
        let company = Share {
            code: company_code.to_string(),
            price: price.clone(),
            price_date: now,
        };
        // println!("{} = {} at {}",
        //          company.code,
        //          company.price,
        //          company.price_date.format("%Y-%m-%d %H:%M:%S").to_string()
        // );

        company_prices.push(company);
    }
    Ok(company_prices)
}

fn print_prices(company_prices: &mut Vec<Share>) {
    let mut tbl = Table::new();
    tbl.add_row(row!["CODE", "PRICE", "TIME"]);
    for company in company_prices {
        tbl.add_row(row![company.code, company.price, company.display_date()]);
    }

    tbl.printstd();
}

fn save_prices(company_prices: &mut Vec<Share>) -> Result<(), Box<dyn std::error::Error>> {
    //db connection
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
    let mut conn = pool.get_conn()?;
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
                        "code" => &company.code,
                        "price" => str::replace(&company.price, ",", "."),
                    }
            ))?;

    Ok(())
}