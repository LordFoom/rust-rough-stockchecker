use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::io::Stdout;

use log::{debug, error};
use termion::clear;
use termion::event::Key;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use tui::{Frame, Terminal, symbols};
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style, Modifier};
use tui::widgets::{Block, Borders, Chart, Dataset, GraphType, Sparkline, Axis};

use crate::share_price_model::{Share, ShareTimeline};
use crate::util::event::{Event, Events};
use tui::text::Span;

pub struct ChartGrapher {
    pub share_data: Vec<ShareTimeline>
}

const DATA: [(f64, f64); 5] = [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0), (4.0, 4.0)];
const DATA2: [(f64, f64); 7] = [
    (0.0, 0.0),
    (10.0, 1.0),
    (20.0, 0.5),
    (30.0, 1.5),
    (40.0, 1.0),
    (50.0, 2.5),
    (60.0, 3.0),
];
impl ChartGrapher {
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
        let mut share_price_arr = HashMap::new();
        for (code, share_prices) in share_data {
            share_price_arr.insert(code.to_string(), share_prices.iter()
                .map(|share| {
                    // debug!("share.price: '{}'", share.price);
                    (share.display_date(), share.price.parse::<f32>().unwrap())
                }).collect::<Vec<(String, f32)>>());
        }

        loop {
            terminal.draw(|f| {
                let size = f.size();
                // let rect = Chart::centered_rect(40, 100, size);

                let chunks = ChartGrapher::get_layout_chunks(size);

                // debug!("About to draw graph");
                for (idx, share_history) in share_price_arr.iter().enumerate() {
                    // debug!("Idx = {}", idx);
                    let share_history_data = &share_history.1.iter()
                        .map(|(date_str, price_float)|
                            (idx as f64, f64::from(*price_float)) ).collect::<Vec<(f64, f64)>>();

                    // debug!("Share price array: {}", share_history_data.len());
                    for (index, price) in share_history_data {
                        // debug!("idx: {}, index: {}, price: {}\n",idx, index, price);
                        // break;
                    }
                    // debug!("ABOUT TO BREAK AAAAH!");
                    // panic!("ABOUT TO BREAK AAAAH!");

                    let share_price_dataset = vec![Dataset::default()
                        .name(share_history.0)
                        .marker(symbols::Marker::Braille)
                        .style(Style::default().fg(Color::Yellow))
                        .graph_type(GraphType::Line)
                        // .data(share_history_data)
                        .data(&DATA)
                    ];

                    let chart = Chart::new(share_price_dataset)
                        .block(
                        Block::default()
                            .title(Span::styled(
                                share_history.0,
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ))
                            .borders(Borders::ALL),
                    ) .x_axis(
                            Axis::default()
                                .title("X Axis")
                                .bounds([0.0, 10.0])
                                .style(Style::default().fg(Color::Gray))
                        ).y_axis(
                        Axis::default()
                            .title("Y Axis")
                            .style(Style::default().fg(Color::Gray))
                            .bounds([0.0, 20.0])
                            // .labels(vec![
                            //     Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                            //     Span::raw("2.5"),
                            //     Span::styled("5.0", Style::default().add_modifier(Modifier::BOLD)),
                            // ]),
                    );
                    // let sparkline = Sparkline::default()
                    //     .block(
                    //         Block::default()
                    //             .title(String::from(share_history.0))
                    //             .borders(Borders::LEFT | Borders::RIGHT),
                    //     )
                    //     .data(share_price_arr)
                    //     .style(Style::default().fg(Color::Cyan));
                    f.render_widget(chart, chunks[idx]);
                    // break;
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