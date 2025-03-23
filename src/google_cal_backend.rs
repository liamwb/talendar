extern crate hyper;
extern crate hyper_rustls;

use std::{collections::HashMap, path::PathBuf, pin::Pin, future::Future};
use chrono::naive::NaiveDate;
use chrono::Local;

use rustls;
use google_calendar3::{api::{CalendarListEntry, Colors, Event as CalendarEvent, EventListCall}, hyper_util, yup_oauth2::{self, authenticator_delegate::{InstalledFlowDelegate, DefaultInstalledFlowDelegate}}, CalendarHub, Error};
use serde::{Deserialize, Serialize};
use localzone;

#[derive(Serialize, Deserialize, Default)]
pub struct CalendarCache {
    // map from calendar ID to corresponding sync token
    sync_tokens: HashMap<String, String>,

    // Store all the events by date
    pub events: HashMap<NaiveDate, Vec<CalendarEvent>>,

    pub calendars: Vec<CalendarListEntry>,

    pub colors: Colors,
}

impl CalendarCache {
    /// add event to `events` hash map, creating a new vec entry if one does not already exist
    fn add_event(&mut self, event: CalendarEvent) {
        let date  = event.get_naive_date().unwrap();
        match self.events.get_mut(&date) {
            // If there is no vec in the hash map, then we need to create one
            None => {
                let vec = vec![event];
                _ = self.events.insert(date, vec);
            }
            // if there is already a vec, then we need to add our event (todo in order?)
            Some(vec) => {
                vec.push(event);
            }
        }
    }

    /// remove event from vec in `events` hash map
    fn remove_event(&mut self, event: CalendarEvent) {
        let date = get_event_naive_date(&event).unwrap_or_default();
        match self.events.get_mut(&date) {
            Some(vec) => {
                vec.retain(|existing_event| {
                    existing_event.id.as_ref().unwrap() != event.id.as_ref().unwrap()
                });
            }
            None => {
                eprintln!("Tried to remove an event already missing from cache");
            }
        }
    }
}

pub struct CalendarClient {
    hub: CalendarHub<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>,
    pub cache: CalendarCache,
    cache_path: PathBuf,
}

impl CalendarClient {
    pub async fn new(cache_path: PathBuf) -> Result<Self, Error> {
        // Create cache directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        } 

        // If the cache exists, load it. If it doesn't exist, create an empty one
        let cache: CalendarCache = if cache_path.exists() {
            let file = std::fs::File::open(&cache_path)
                .expect("Error loading the cache");
            serde_json::from_reader(file).unwrap_or_default()
        } else {
            CalendarCache::default()
        };

        let hub = initialize_calendar_hub(&cache_path);

        Ok( Self{ 
            hub: hub.await,
            cache,
            cache_path,
        })
    }

    fn save_cache(&self) -> Result<(), Error> {
        let file = std::fs::File::create(&self.cache_path)
            .expect("Error creating the cache file");

        _ = serde_json::to_writer_pretty(file, &self.cache);

        Ok(())
    }

    /// Fetches the calendar list from the google calendar api, and calls incremental_sync_events
    /// on each calendar therin to sync all of the events in each calendar.
    /// Fetches the Colors.
    pub async fn sync(&mut self) -> Result<(), google_calendar3::Error> {
        let calendars = self.hub.calendar_list()
            .list()
            .doit()
            .await?
            .1  // doit() returns Result<(Response, CalendarList)>
            .items
            .unwrap_or_default();

        // Sync the calendar list
        self.cache.calendars = calendars.clone();

        // Sync the events in each calendar
        for calendar in calendars {
            self.incremental_sync_events(&calendar).await?;
        }

        // Fetch the colors
        let _ = self.fetch_colors().await?;

        self.save_cache()

    }

    /// incremental_sync_events
    /// -- calls EventListCall::sync_token to get the latest sync token
    /// -- updates self.cache.events accordingly
    /// -- stores the new sync token for next time
    async fn incremental_sync_events(&mut self, calendar: &CalendarListEntry) -> Result<(), google_calendar3::Error> {
        let calendar_id = calendar.id.clone().unwrap();
        let calendar_id_str = calendar_id.as_str();
        let local_timezone = localzone::get_local_zone().unwrap_or("Australia/Melbourne".to_string());

        let get_request = || { self.hub
            .events()
            .list(calendar_id_str)
            .single_events(true)
            .time_zone(&local_timezone)
            .max_results(2500)};


        let mut request: EventListCall<'_, hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

        // use sync token if we have one
        if let Some(token) = self.cache.sync_tokens.get(calendar_id_str) {
            request = get_request().sync_token(token);
        } else {
            request = get_request();
        }

        let mut response = request.doit().await?.1;

        // loop through all the pages
        loop {
            // Handle the events we received
            if let Some(events) = response.items {
                for event in events {

                    if let Some(_date) = get_event_naive_date(&event) {

                        if let Some(status) = &event.status {
                            match status.as_str() {
                                "cancelled" => self.cache.remove_event(event) ,
                                _ => self.cache.add_event(event),
                            }
                        } else {
                            self.cache.add_event(event);
                        }
                    }
                }
                // handle pagination
                match response.next_page_token {
                    Some(token) => {
                        // use sync token if we have one
                        if let Some(token) = self.cache.sync_tokens.get(calendar_id_str) {
                            request = get_request().sync_token(token);
                        } else {
                            request = get_request();
                        }
                        request = request.page_token(&token);
                        response = request.doit().await?.1;
                    }
                    None => break
                };

            } else {
                // if we didn't receive any events then we're done
                break
            }
        }


        // Store the new sync token
        if let Some(token) = response.next_sync_token {
            self.cache.sync_tokens.insert(calendar_id, token);
        };

        Ok(())
    }

    pub fn get_calendars(&self) -> &[CalendarListEntry] {
        &self.cache.calendars
    }

    pub fn get_events_by_date(&self, date: &NaiveDate) -> Option<&Vec<CalendarEvent>> {
        self.cache.events.get(date)
    }

    /////////////
    // Colours //
    /////////////

    async fn fetch_colors(&mut self) -> Result<(), google_calendar3::Error>{
        let (_, colors) = self.hub.colors().get().doit().await?;
        self.cache.colors = colors;
        Ok(())
    }

}


pub async fn initialize_calendar_hub(cache_path: &PathBuf) -> CalendarHub<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>> {
    // Read application secret from a file. Sometimes it's easier to compile it directly into
    // the binary. The clientsecret file contains JSON like `{"installed":{"client_id": ... }}`
    let secret = yup_oauth2::read_application_secret("clientsecret.json")
        .await
        .expect("clientsecret.json");

    // Create an authenticator that uses an InstalledFlow to authenticate. The
    // authentication tokens are persisted to a file named tokencache.json. The
    // authenticator takes care of caching tokens to disk and refreshing tokens once
    // they've expired.
    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
        .persist_tokens_to_disk(cache_path.to_str().unwrap().to_owned() + "tokenfile.json")
        .flow_delegate(Box::new(InstalledFlowBrowserDelegate))
        .build()
        .await
        .expect("Failed to create Authenticator");

    // It seems that rustls have changed their crypto provider api to support different
    // providers and use aws_lc_rs by default, but hyper_rustls has not been updated to reflect
    // this change. Installing the aws_lc_rs provider explicitly prevents a panic, and it's
    // what should happen automatically anyway (I think).
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    
    let client = hyper_util::client::legacy::Client::builder(
        hyper_util::rt::TokioExecutor::new()
    )
        .build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .unwrap()
                .https_or_http()
                .enable_http1()
                .build()
        );
    let hub = CalendarHub::new(client, auth);

    hub
}

pub trait CalendarEventExt {
    fn is_multiday(&self) -> bool;

    fn get_naive_date(&self) -> Option<NaiveDate>;

    fn get_naive_end_date(&self) -> Option<NaiveDate>;

    fn get_start_string(&self) -> String;
}

impl CalendarEventExt for CalendarEvent {
    /// Returns `true` if `self` has both a start and end date, and the number of days in the
    /// difference is greater than one. Returns `false` otherwise.
    fn is_multiday(&self) -> bool {
        if let ( Some(start_date), Some(end_date) ) = ( self.get_naive_date(),  self.get_naive_end_date() ) {
            return (end_date - start_date).num_days() <= 1
        } else {
            return false
        }
    }

    fn get_naive_date(&self) -> Option<NaiveDate> {
        match &self.start {
            Some(event_date_time) => {
                if let Some(date) = event_date_time.date {
                    Some(date)
                } else if let Some(date_time) = event_date_time.date_time {
                    Some(date_time.with_timezone(&Local).date_naive())
                } else {
                    None
                }
            }
            None => None
        }
    }

    fn get_naive_end_date(&self) -> Option<NaiveDate> {
        match &self.end {
            Some(event_date_time) => {
                if let Some(date) = event_date_time.date {
                    Some(date)
                } else if let Some(date_time) = event_date_time.date_time {
                    Some(date_time.naive_local().date())
                } else {
                    None
                }
            }
            None => None
        }
    }

    /// Get the start time of the event. If the event all-day there will be no start time, and the
    /// result will have "ALL DAY" appended. If the event is not all day then the string
    /// representation its date_time will be returned
    fn get_start_string(&self) -> String {
        match &self.start {
            Some(event_date_time) => {
                // First check in date_time as it contains more info than date
                if let Some(date_time) = event_date_time.date_time {
                    date_time.with_timezone(&Local).to_string()
                }
                // if date_time is empty, then check for a date
                else if let Some(date) = event_date_time.date {
                    date.to_string() + " ALL DAY"
                }
                // if all else fails, return some informative default
                else{
                    String::from("No Start Time")
                }
            }
            None => {
                String::from("No Start Time")
            }
        }
    }
}

/// Returns the NaiveDate of event, or None if event.date does not contain a NaiveDate 
fn get_event_naive_date(event: &CalendarEvent) -> Option<NaiveDate> {
    match &event.start {
        Some(event_date_time) => {
            if let Some(date) = event_date_time.date {
                Some(date)
            } else if let Some(date_time) = event_date_time.date_time {
                Some(date_time.naive_local().date())
            } else {
                None
            }
        }
        None => None
    }
}

////////////////////////////////////////////////////////////
//                                                        //
// Example Implementation of custom InstalledFlowDelegate //
//                                                        //
////////////////////////////////////////////////////////////

// The following code is taken from the yup_oauth2 examples: https://github.com/dermesser/yup-oauth2/blob/52e29d8db1cd91e6074d6f589bf586220ad05ec4/examples/custom_flow.rs
/// async function to be pinned by the `present_user_url` method of the trait
/// we use the existing `DefaultInstalledFlowDelegate::present_user_url` method as a fallback for
/// when the browser did not open for example, the user still see's the URL.
async fn browser_user_url(url: &str, need_code: bool) -> Result<String, String> {
    if webbrowser::open(url).is_ok() {
        println!("webbrowser was successfully opened.");
    }
    let def_delegate = DefaultInstalledFlowDelegate;
    def_delegate.present_user_url(url, need_code).await
}

/// our custom delegate struct we will implement a flow delegate trait for:
/// in this case we will implement the `InstalledFlowDelegated` trait
#[derive(Copy, Clone)]
struct InstalledFlowBrowserDelegate;

/// here we implement only the present_user_url method with the added webbrowser opening
/// the other behaviour of the trait does not need to be changed.
impl InstalledFlowDelegate for InstalledFlowBrowserDelegate {
    /// the actual presenting of URL and browser opening happens in the function defined above here
    /// we only pin it
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(browser_user_url(url, need_code))
    }
}



///////////////////////////////////////////
//                                       //
// Custom ratatui InstalledFlowDelegate  //
//                                       //
///////////////////////////////////////////
#[derive(Copy, Clone)]
struct InstalledFlowBrowserDelegateRatatui;

impl InstalledFlowDelegate for InstalledFlowBrowserDelegateRatatui {

    fn present_user_url<'a>(
            &'a self,
            url: &'a str,
            need_code: bool,
        ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        todo!()
    }
}
