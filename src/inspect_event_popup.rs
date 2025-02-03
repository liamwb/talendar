use ratatui::{
    Frame,
    widgets::{Block, Clear},
    prelude::{Rect, Layout, Constraint},
    layout::Flex,
};
use google_calendar3::api::Event as CalendarEvent;



pub fn draw_inspect_event_popup(frame: &mut Frame, event: CalendarEvent) {
    let block = Block::bordered().title("Popup");

    let area = frame.area();
    let area = popup_area(area, 80, 80);
    frame.render_widget(Clear, area); // this clears out the background
    frame.render_widget(block, area);
}


/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
