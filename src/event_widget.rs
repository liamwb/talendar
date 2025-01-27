// Define an event widget
use google_calendar3::api::{Colors, Event as CalendarEvent};
use ratatui::{
    buffer::Buffer, 
    layout::Rect, 
    text::Span, 
    style::Style,
    prelude::{Line, Text},
    widgets::Widget};

use crate::utils::get_event_color;

const DEFAULT_TITLE : &str = "No title";
const DEFAULT_COLOR : &str = "#FFFFFF";


pub struct EventWidget<'a> {
    event: &'a CalendarEvent,
    colors : &'a Colors,
}

impl<'a> Widget for EventWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // TODO: This does not appear to work for some reason
        let color = get_event_color(self.event, self.colors);

        let default_title = String::from("No title");
        let title = self.event.summary.as_ref().unwrap_or(&default_title);

        let line = Line::from(vec![
            Span::styled("• ", Style::default().fg(color)),
            Span::styled(title, Style::default())
        ]);
        // buf.set_line(0, 0, &line, area.width);
        line.render(area, buf);
    }
}

// We want this so that we can populate the List widget with EventWidgets
impl<'a> Into<Text<'a>> for &EventWidget<'a> {
    fn into(self) -> Text<'a> {
        let color_hex = String::from( self.event.color_id.as_ref().unwrap_or(&String::from(DEFAULT_COLOR)) );
        let color = serde_json::from_str(&color_hex).unwrap_or_default();
        let title = String::from(self.event.summary.as_ref().unwrap_or(&String::from(DEFAULT_TITLE)));

        let line = Line::from(vec![
            Span::styled("• ", Style::default().fg(color)),
            Span::styled(title, Style::default())
        ]
        );

        Text::from(line)
    }
}


impl<'a> EventWidget<'a> {
    pub fn new(event: & 'a CalendarEvent, colors: &'a Colors) -> Self {
        EventWidget { event , colors }
    }
}
