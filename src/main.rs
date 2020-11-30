extern crate prettytable;

use std::borrow::Borrow;
use std::collections::HashMap;
use std::string::ToString;

use chrono::prelude::*;
use clap::{App, Arg, ArgMatches};
use mysql::{OptsBuilder, params, Pool, PooledConn};
use mysql::prelude::*;
use prettytable::{Attr, Cell, color, Row, Table};
use regex::Regex;
use rust_decimal::Decimal;
use select::document::Document;
use select::predicate::Name;

use crate::share_price_model::{Share, ShareMoment, ShareTimeline};
use rust_decimal::prelude::Zero;

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
    let company_prices = get_company_prices(company_codes).await?;
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
                && starts_with_digits.is_match(&txt) {
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

        if let Some(share) = load_share_history(company_code, 1) {
            share_history.insert(ShareMoment::Yesterday, share);
        }
        if let Some(share) = load_share_history(company_code, 7) {
            share_history.insert(ShareMoment::LastWeek, share);
        }
        if let Some(share) = load_share_history(company_code, 30) {
            share_history.insert(ShareMoment::LastMonth, share);
        }


        let share_timeline = ShareTimeline {
            share: company_curr,
            share_history,
        };
        company_prices.push(share_timeline);
    }
    Ok(company_prices)
}

fn load_share_history(company_code: &str, days_ago_upper_limit: i32) -> Option<Share> {
    let mut conn = get_db_connection().unwrap();
    let base_select = r"SELECT company_code as code, price, price_date
                         FROM stock_prices WHERE company_code = :code
                         AND date(price_date) <= curdate() ";
    let yest_select = format!("{} - INTERVAL {} DAY ORDER BY id DESC ", base_select, days_ago_upper_limit);
    match conn.exec_first(yest_select,
                          params! {"code"=>company_code}) {
        Ok(Some((code, price, price_date))) => Some(Share { code, price, price_date }),
        Ok(None) => None,
        Err(e) => panic!("Unable to get previous company info: {}", e.to_string()),
    }
}

fn construct_current_moment_share_columns(share: &share_price_model::Share) -> Vec<Cell> {
    vec![
        Cell::new(&share.code),
        Cell::new(&share.pretty_price()).with_style(Attr::ForegroundColor(color::BRIGHT_BLUE)),
        Cell::new(&share.display_date()),
    ]
}

fn construct_historic_moment_share_columns(share_history_optional: Option<&share_price_model::Share>, share: &share_price_model::Share) -> Vec<Cell> {
    match share_history_optional {
        Some(share_history) => construct_non_default_historic_row_section(share_history, share),
       None => vec![Cell::new("---"),Cell::new("---"),Cell::new("---"),Cell::new("---") ],
    }

    //in theory we've taken care of the None just above.....
}

fn construct_non_default_historic_row_section(share_history: &Share, share: &Share) -> Vec<Cell> {
//calculate the price difference
    let curr_price = share.price_as_decimal();
    let historic_price = share_history.price_as_decimal();
    let movement = curr_price - historic_price;

    //calculate percentage price change
    let percentage_movement = (curr_price / historic_price) * Decimal::from(100) - Decimal::from(100);
    let (movement_style, percentage_string) = if movement < Decimal::zero() {
        (Attr::ForegroundColor(color::RED), format!("{:.2}%", percentage_movement))
    } else {
        (Attr::ForegroundColor(color::GREEN), format!("+{:.2}%", percentage_movement))
    };

    vec![
        Cell::new(&share_history.pretty_price()).with_style(Attr::ForegroundColor(color::BRIGHT_BLUE)),
        Cell::new(&share_history.display_date()),
        Cell::new(&movement.to_string())
            .with_style(Attr::Bold)
            .with_style(movement_style),
        Cell::new(&percentage_string)
            .with_style(Attr::Bold)
            .with_style(movement_style),
    ]
}

fn print_prices(company_prices: &Vec<ShareTimeline>) {
    let mut tbl = Table::new();
    let header_vec = construct_table_header();
    tbl.add_row(Row::new(header_vec));

    for share_timeline in company_prices {
        let mut share_row:Vec<Cell> = Vec::new();
        share_row.append(&mut construct_current_moment_share_columns(&share_timeline.share));

        share_row.append(&mut construct_historic_moment_share_columns(share_timeline.share_history.get(&ShareMoment::Yesterday), &share_timeline.share));
        share_row.append(&mut construct_historic_moment_share_columns(share_timeline.share_history.get(&ShareMoment::LastWeek), &share_timeline.share));
        share_row.append(&mut construct_historic_moment_share_columns(share_timeline.share_history.get(&ShareMoment::LastMonth), &share_timeline.share));
        share_row.append(&mut construct_historic_moment_share_columns(share_timeline.share_history.get(&ShareMoment::LastYear), &share_timeline.share));

        tbl.add_row(Row::new(share_row));

    }

    tbl.printstd();
}

fn construct_table_header() -> Vec<Cell> {
    let mut header_vec = construct_default_headers();
    header_vec.append(&mut construct_price_cell_headers(&ShareMoment::Yesterday));
    header_vec.append(&mut construct_price_cell_headers(&ShareMoment::LastWeek));
    header_vec.append(&mut construct_price_cell_headers(&ShareMoment::LastMonth));
    header_vec.append(&mut construct_price_cell_headers(&ShareMoment::LastYear));
    header_vec
}


fn construct_default_headers() -> Vec<Cell> {
    vec![
        Cell::new("CODE")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::BLUE))
        ,
        Cell::new("CURRENT \nPRICE")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::YELLOW))
        ,
        Cell::new("CURR \nTIME")
            .with_style(Attr::Bold)
            .with_style(Attr::ForegroundColor(color::YELLOW))
        ,
    ]
}

//should I return a Vec?? Probably....
fn construct_price_cell_headers(share_history: &share_price_model::ShareMoment) -> Vec<Cell> {
    let str_hist = share_history.to_string();
    vec![
        make_header(&format!("{} \nPRICE", str_hist), color::BRIGHT_BLUE),
        make_header(&format!("{} \nDATE", str_hist), color::BRIGHT_BLUE),
        make_header(&format!("{} \nMOVEMENT", str_hist), color::BRIGHT_YELLOW),
        make_header("", color::BRIGHT_YELLOW),
    ]
}

fn make_header(col_name: &str, color: color::Color) -> Cell {
    Cell::new(col_name)
        .with_style(Attr::Bold)
        .with_style(Attr::ForegroundColor(color))
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