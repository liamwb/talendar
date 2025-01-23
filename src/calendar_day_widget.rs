use chrono::{Datelike, NaiveDate};
use ratatui::{
    buffer::Buffer, 
    layout::Rect, 
    widgets::{Block, List, Widget}
};
use crate::event_widget::EventWidget;

pub struct CalendarDayWidget<'a> {
    events: Vec<EventWidget<'a>>,
    date: &'a NaiveDate
}

impl Widget for CalendarDayWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) { 
        let mut title = self.date.day().to_string();
        if &chrono::offset::Local::now().date_naive() == self.date {
            title += " [Today]"
        }

        let block = Block::bordered().title(title);
        let list = List::new(&self.events).block(block);

        list.render(area, buf);
    }
}

impl<'a> CalendarDayWidget<'a> {
    pub fn new(events: Vec<EventWidget<'a>>, date: &'a NaiveDate) -> Self {
        CalendarDayWidget { events , date }
    }
}
