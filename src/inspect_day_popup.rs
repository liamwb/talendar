use ratatui::{
    layout::Flex, prelude::{Constraint, Layout, Rect}, widgets::{Block, Clear, Table, Row}, Frame, 
    };
use google_calendar3::api::Event as CalendarEvent;
use std::iter::zip;

use crate::google_cal_backend::CalendarEventExt;



pub fn draw_inspect_day_popup(frame: &mut Frame, events: Option<&Vec<CalendarEvent>>) {
    let block = Block::bordered().title("Popup");
    let area = frame.area();
    let area = popup_area(area, 80, 80);
    frame.render_widget(Clear, area); // this clears out the background
    
    // unpack the events, if there are none use an empty vec
    let events_vec: &Vec::<CalendarEvent> = match events {
        Some(v) => v.as_ref(),
        None => {  // if there are no events that day, empty table
            let table = Table::default().block(block);
            frame.render_widget(table, area);
            return
        }
    };

    let event_titles = events_vec.into_iter()
        .map(
            |event| event.summary.clone().unwrap_or_default()
        )
        .collect::<Vec<String>>();

    let event_start_strings = events_vec.into_iter()
        .map(
            |event| event.get_start_string()
        );

    let rows = zip(event_titles, event_start_strings).map(
        |(title, start_string)| Row::new([title, start_string])
    )
        .collect::<Vec<Row>>();

    let widths = vec![
        Constraint::Ratio(1, rows.len().try_into().unwrap())
        ; rows.len()
    ];
    
    // draw a table with events in it
    let table = Table::new(rows, widths).block(block);

    frame.render_widget(table, area);
}


/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
