use crossterm;
use google_calendar3::api::{Colors, Event as CalendarEvent};
use ratatui::style;
use std::str::FromStr;


const DEFAULT_COLOR: &str  = "#FFFFFF";

pub fn month_to_str(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => {
            eprintln!("Month must be in [1, 12] but it was {}", month);
            panic!();
        }
    }
}

pub fn get_event_color<'a>(event: &CalendarEvent, colors: &'a Colors) -> style::Color {
    let color_id = if let Some(color_id) = &event.color_id {
        color_id
    } else {
        return style::Color::from_str(DEFAULT_COLOR).unwrap()
    };

    let mut color = colors.event
        // event is Option<Hashmap>, we want to lookup in &HashMap
        .as_ref().and_then(|event| event.get(color_id.as_str()))
        // get() returns Option<&ColorDefinition>, we want &foreground
        .and_then(|color_definition| color_definition.foreground.as_ref())
        // foreground is Option<&String>, convert to Option<&str>
        .map(|foreground| foreground.as_str());

    let style_color = if let Some(color) = color {
        style::Color::from_str(color)
    } else {
        style::Color::from_str(DEFAULT_COLOR)
    };

    match style_color {
        Ok(res) => res,
        _ => style::Color::from_str(DEFAULT_COLOR).unwrap()
    }
}

