use chrono::{Datelike, NaiveDate};
use ratatui::{
    buffer::Buffer, 
    layout::Rect, 
    widgets::{Block, List, ListState, StatefulWidget, Widget}
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

        // render is implemented for both StatefulWidget and Widget, so we need to tell
        // Rust which one to use explicitly. This is equivalent to list.render(area, buf)
        Widget::render(list, area, buf);
    }
}

impl<'a> CalendarDayWidget<'a> {
    pub fn new(events: Vec<EventWidget<'a>>, date: &'a NaiveDate) -> Self {
        CalendarDayWidget { events , date }
    }
}

// Extending the CalendarDayWidget so it can store some state:
// - is it selected?
// - which event is selected, if an event is ndidate #2 is defined in an impl of the trait `Widget`selected?

pub struct CalendarDayWidgetState {
    is_selected: bool,
    list_state: ListState,
}

impl StatefulWidget for CalendarDayWidget<'_> {
    type State = CalendarDayWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        todo!()
    }
}
