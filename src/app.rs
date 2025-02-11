use chrono::{Datelike, Days, Months, NaiveDate};
use color_eyre::{Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint::{self}, Direction, Layout, Rect}, text::Text, DefaultTerminal, Frame
};
use google_calendar3::api::{CalendarListEntry, Event as CalendarEvent};
use std::path::PathBuf;
use::directories::ProjectDirs;
use std::fs;

use crate::{calendar_day_widget::{CalendarDayWidget, CalendarDayWidgetState}, event_widget::EventWidget, google_cal_backend::CalendarClient, inspect_day_popup::draw_inspect_day_popup};
use crate::utils::month_to_str;

pub struct App {
    /// Is the application running?
    running: bool,
    #[allow(dead_code)]
    view: CalendarView,
    calendar_client: CalendarClient,
    /// calendars (ids) to display to the user
    #[allow(dead_code)]
    active_calendars: Vec<(CalendarListEntry, bool)>,

    // State
    currently_selected_date: NaiveDate,
    // [selected day, other days]
    calendar_view_state: (CalendarDayWidgetState, CalendarDayWidgetState),

    show_inspect_day_popup: bool,
}

#[derive(Debug, Default)]
enum CalendarView {
    #[default]
    MONTH,
    _WEEK,
    _DAY,
}


fn determine_cache_path() -> PathBuf {
    if let Some(project_dir) = ProjectDirs::from("", "", "talendar") {
        let data_dir = project_dir.data_dir();

        fs::create_dir_all(data_dir).expect("Failed to create data directory");

        let mut path = data_dir.to_path_buf();
        path.push("cache.json");
        path

    } else {
        panic!("Failed to crete data directory");
    }
}

impl App {
    /// Construct a new instance of [`App`].
    pub async fn new() -> Self {
        // data directory for us to persist out calendar data
        let cache_path = determine_cache_path();

        // connect to google calendar api by creating a new CalendarClient
        let calendar_client = CalendarClient::new(cache_path).await.unwrap();

        
        // enable all calendars by default
        // note that we're cloning the list of calendars, which means we need to remember to update
        // the ui if we add/remove calendars
        let active_calendars = calendar_client
            .get_calendars()
            .iter()
            .cloned()
            .map(|cal| (cal, true))
            .collect();

        let mut new_app = Self { 
            running: true, 
            view: CalendarView::MONTH, 
            calendar_client,
            active_calendars,
            currently_selected_date: chrono::offset::Local::now().date_naive(),
            calendar_view_state: (CalendarDayWidgetState::default(), CalendarDayWidgetState::default()),
            show_inspect_day_popup: false
        };

        let _ = new_app.calendar_client.sync().await;

        new_app
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/master/examples>
    fn draw(&mut self, frame: &mut Frame) {
        self.draw_month_view(frame);

        if self.show_inspect_day_popup {
            let events = self.calendar_client.get_events_by_date(&self.currently_selected_date);
            draw_inspect_day_popup(frame, events)
        }
    }

    fn draw_month_view(&mut self, frame: &mut Frame) {
        // split the area into header and body
        let header_body_areas = Layout::vertical([
            Constraint::Max(2),
            Constraint::Fill(1)
        ])
        .split(frame.area());
        self.draw_header(frame, header_body_areas[0]);

        // the month is a 5x7 grid of CalendarDayWidgets
        let row_constraints = Constraint::from_percentages(vec![20; 5]);
        let col_constraints = Constraint::from_percentages(vec![100/7; 7]);

        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(header_body_areas[1]);

        let mut areas: [ [ Rect; 7 ] ; 5 ] = [ [ Rect::default(); 7]; 5];

        // fill the areas array with the appropriate Rects
        for (i, row) in row_areas.into_iter().enumerate() {
            let col_areas = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(col_constraints.clone())  // need a clone for each row
                .split(*row);

            for (j, area) in col_areas.into_iter().enumerate() {
                areas[i][j] = *area;
            }
        }

        // start at the first day of the month of the selected date
        // TODO use from_ymp_opt instead
        let mut date = self.currently_selected_date.clone().with_day(1)
            .unwrap_or(NaiveDate::from_ymd(2025, 1, 1));

        // we want the first calendar_day to be the previous monday (or date if date is a monday)
        // TODO: choose between Monday and Sunday starting weeks
        date = date -  chrono::TimeDelta::days(date.weekday().num_days_from_monday().into());

        // render the calendar day widgets
        for (_i, row) in areas.into_iter().enumerate() {
            for (_j, area) in row.into_iter().enumerate() {
                let calendar_events = match self.calendar_client.get_events_by_date(&date) {
                    Some(calendar_events) => calendar_events,
                    None => &Vec::<CalendarEvent>::new()
                };

                let event_widgets: Vec<EventWidget> = calendar_events.iter().map(|cal_event| {
                    EventWidget::new(
                        cal_event, 
                        &self.calendar_client.cache.colors
                    )
                })
                    .collect();

                let is_selected = { date == self.currently_selected_date };
                let state: &mut CalendarDayWidgetState = if is_selected { self.calendar_view_state.0.state_mut() } else { self.calendar_view_state.1.state_mut() };
                // update the selection status of the appropriate calendar_view_state 
                // TODO this is not efficient, we only see the selected date once per outer loop
                if is_selected && !state.is_selected() {
                    state.set_selected(true)
                } else if !is_selected && *state.is_selected() {
                    state.set_selected(false)
                }

                frame.render_stateful_widget(
                    CalendarDayWidget::new(
                        event_widgets, 
                        &date
                    ),
                    area,
                    state
                );

                // TODO: Use succ_opt instead
                date = date.succ();
            }
        }


    }

    fn draw_header(&mut self, frame: &mut Frame, rect: Rect) {
        let constraints = Constraint::from_percentages([20, 60, 20]);
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(rect);
        let month = month_to_str(self.currently_selected_date.month());
        let year = self.currently_selected_date.year().to_string();


        let title = Text::from(format!("{} {}", month, year))
            .centered();
        let previous = Text::from("[p]revious")
            .left_aligned();
        let next = Text::from("[n]ext")
            .right_aligned();

        frame.render_widget(title, layout[1]);
        frame.render_widget(previous, layout[0]);
        frame.render_widget(next, layout[2]);
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's importantpanto check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            (_, KeyCode::Char('p')) => self.currently_selected_date = self.currently_selected_date - Months::new(1),
            (_, KeyCode::Char('n')) => self.currently_selected_date = self.currently_selected_date + Months::new(1),
            (_, KeyCode::Char('l')) => self.next_day(),
            (_, KeyCode::Char('h')) => self.previous_day(),
            (_, KeyCode::Char('j')) => self.next_week(),
            (_, KeyCode::Char('k')) => self.previous_week(),
            (_, KeyCode::Char('i')) => self.show_inspect_day_popup = !self.show_inspect_day_popup,
            _ => {}
        }
    }

    /// sets [`self.currently_selected_date`] to the next day
    fn next_day(&mut self) {
        self.currently_selected_date = self.currently_selected_date + Days::new(1)
    }

    /// sets [`self.currently_selected_date`] to the previous day
    fn previous_day(&mut self) {
        self.currently_selected_date = self.currently_selected_date - Days::new(1)
    }

    /// sets [`self.currently_selected_date`] to the next week
    fn next_week(&mut self) {
        self.currently_selected_date = self.currently_selected_date + Days::new(7)
    }

    /// sets [`self.currently_selected_date`] to the previous week
    fn previous_week(&mut self) {
        self.currently_selected_date = self.currently_selected_date - Days::new(7)
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }


}


////////////////////////////////////////////
//                                        //
// Define some example events for testing //
//                                        //
////////////////////////////////////////////

