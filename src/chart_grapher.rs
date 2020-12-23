use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::Stdout;

use termion::clear;
use termion::event::Key;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use tui::{Frame, Terminal};
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Sparkline};

use crate::share_price_model::{Share, ShareTimeline};
use crate::util::event::{Event, Events};
use log::{debug, error};

pub struct Chart {
    pub share_data: Vec<ShareTimeline>
}

impl Chart {
    /// helper function to create a centered rect using up
    /// certain percentage of the available rect `r`
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Percentage(percent_y),
                    Constraint::Percentage((100 - percent_y) / 2),
                ]
                    .as_ref(),
            )
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                    .as_ref(),
            )
            .split(popup_layout[1])[1]
    }

    pub fn draw_graph(share_data: &HashMap<String, Vec<Share>>) -> Result<(), Box<dyn Error>> {
        let stdout = io::stdout().into_raw_mode()?;
        // let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // println!("{}", clear::All);

        // Setup event handlers
        let events = Events::new();
        let mut share_price_arr:HashMap<String, Vec<f32>> = HashMap::new();
        for (code, share_prices) in share_data {
            share_price_arr.insert(code.to_string() , share_prices.iter()
                .map(|share| {
                    debug!("share.price: '{}'", share.price);
                    share.price.parse::<f32>().unwrap()
                } )
                .collect::<Vec<f32>>());
        }

        loop {
            terminal.draw(|f| {
                let size = f.size();
                // let rect = Chart::centered_rect(40, 100, size);

                let chunks = Chart::get_layout_chunks(size);

                for (idx,share_history) in share_price_arr.iter().enumerate() {
                    let share_price_arr = &share_history.1.iter().map(|price_float| price_float.round() as u64).collect::<Vec<u64>>();
                    let sparkline = Sparkline::default()
                        .block(
                            Block::default()
                                .title(String::from(share_history.0))
                                .borders(Borders::LEFT | Borders::RIGHT),
                        )
                        .data(share_price_arr)
                        .style(Style::default().fg(Color::Cyan));
                    f.render_widget(sparkline, chunks[idx]);
                };

            })?;

            match events.next()? {
                Event::Input(input) => {
                    if input == Key::Char('q') {
                        break;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn get_layout_chunks(area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(7),
                    Constraint::Min(0),
                ]
                    .as_ref(),
            ).split(area)
    }
}