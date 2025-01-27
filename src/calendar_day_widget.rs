use chrono::{Datelike, NaiveDate};
use color_eyre::owo_colors::OwoColorize;
use ratatui::{
    buffer::Buffer, layout::Rect, style::{Color, Style, Stylize}, widgets::{Block, List, ListState, StatefulWidget, Widget}
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
// - which event is selected, if an event is selected

pub struct CalendarDayWidgetState {
    is_selected: bool,
    list_state: ListState,
}

impl CalendarDayWidgetState {
    pub fn default() -> CalendarDayWidgetState {
        CalendarDayWidgetState{ is_selected: false, list_state: ListState::default() }
    }

    pub fn state_mut(&mut self) -> &mut Self {
        self
    }

    pub fn list_state_mut(&mut self) -> &mut ListState {
        &mut self.list_state
    }
}

impl StatefulWidget for CalendarDayWidget<'_> {
    type State = CalendarDayWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut title = self.date.day().to_string();
        if &chrono::offset::Local::now().date_naive() == self.date {
            title += " [Today]";
            // TODO temporary for testing
            state.is_selected = true;
        }

        let mut block = Block::bordered().title(title);

        // if this widget is selected, then highlight it
        if state.is_selected == true {
            block = block.border_style(Style::default().fg(Color::LightBlue));
        }

        let list = List::new(&self.events).block(block);


        // render is implemented for both StatefulWidget and Widget, so we need to tell
        // Rust which one to use explicitly. This is equivalent to list.render(area, buf, state)
        StatefulWidget::render(list, area, buf, state.list_state_mut())
    }
}
