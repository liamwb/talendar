// TODO Remove this and fix everything
#![allow(deprecated)]

pub use app::App;

pub mod app;
pub mod google_cal_backend;
mod event_widget;
mod calendar_day_widget;
mod utils;
mod inspect_day_popup;


#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();

    let result = App::new().await.run(terminal);
    ratatui::restore();
    result
}


