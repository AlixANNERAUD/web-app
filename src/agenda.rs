use crate::{prelude::*, slider};

fn format_day(day_name: Weekday, day: u32) -> String {
    let day_name = t(match day_name {
        Weekday::Mon => "Lundi",
        Weekday::Tue => "Mardi",
        Weekday::Wed => "Mercredi",
        Weekday::Thu => "Jeudi",
        Weekday::Fri => "Vendredi",
        Weekday::Sat => "Samedi",
        Weekday::Sun => "Dimanche",
    });

    format!("{} {}", day_name, day)
}

pub struct Agenda {
    selected_day: Date<chrono_tz::Tz>,
    events: Vec<RawEvent>,
    slider: Rc<RefCell<slider::SliderManager>>,
    announcements: Vec<AnnouncementDesc>,
    pub displayed_announcement: Option<AnnouncementDesc>,
    selected_event: Rc<Option<RawEvent>>,
    user_info: Option<UserInfo>,
}

pub enum AgendaMsg {
    ScheduleSuccess(Vec<RawEvent>),
    ScheduleFailure(ApiError),
    Previous,
    Next,
    Goto {day: u32, month: u32, year: i32},
    SetSelectedEvent(Option<common::Event>),
    CloseAnnouncement,
    AnnouncementsSuccess(Vec<AnnouncementDesc>),
    Refresh,
    FetchColors(HashMap<String, String>),
    ApiFailure(ApiError),
    PushColors()
}

#[derive(Properties, Clone)]
pub struct AgendaProps {
    pub app_link: Scope<App>,
    pub user_info: Option<UserInfo>,
}

impl PartialEq for AgendaProps {
    fn eq(&self, _other: &Self) -> bool { true }
}

impl Component for Agenda {
    type Message = AgendaMsg;
    type Properties = AgendaProps;

    fn create(ctx: &Context<Self>) -> Self {
        let now = chrono::Local::now();
        let now = now.with_timezone(&Paris);

        // Update data
        let events = init_events(now, ctx.link().clone());
        let announcements = init_announcements(now, ctx.link().clone());
        let displayed_announcement = select_announcement(&announcements, &ctx.props().user_info.clone());

        // Trigger color sync when page is closed
        let link = ctx.link().clone();
        let unload = Closure::wrap(Box::new(move |_: web_sys::Event| {
            link.send_message(AgendaMsg::PushColors());
        }) as Box<dyn FnMut(_)>);
        window().add_event_listener_with_callback("unload", unload.as_ref().unchecked_ref()).unwrap();
        unload.forget();

        // Get colors
        crate::COLORS.fetch_colors(ctx);

        // Auto-push colors every 15s if needed
        let link = ctx.link().clone();
        let push_colors = Closure::wrap(Box::new(move || {
            link.send_message(AgendaMsg::PushColors());
        }) as Box<dyn FnMut()>);
        if let Err(e) = window().set_interval_with_callback_and_timeout_and_arguments(push_colors.as_ref().unchecked_ref(), 1000*15, &Array::new()) {
            sentry_report(JsValue::from(&format!("Failed to set timeout: {:?}", e)));
        }
        push_colors.forget();

        // Switch to next day if it's late or to monday if it's weekend
        let weekday = now.weekday();
        let curr_day = now.naive_local().date().and_hms(0, 0, 0);
        let has_event = has_event_on_day(&events, curr_day, Weekday::Sat);
        if now.hour() >= 19 || weekday == Weekday::Sun || (weekday == Weekday::Sat && !has_event) {
            let link2 = ctx.link().clone();
            spawn_local(async move {
                sleep(Duration::from_millis(500)).await;
                link2.send_message(AgendaMsg::Next);
            });
        }
        
        Self {
            events,
            selected_day: now.date(),
            slider: slider::SliderManager::init(ctx.link().clone(), -20 * (now.date().num_days_from_ce() - 730000)),
            announcements,
            displayed_announcement,
            selected_event: Rc::new(None),
            user_info: ctx.props().user_info.clone(),
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        if self.user_info == ctx.props().user_info {
            false
        } else {
            self.user_info = ctx.props().user_info.clone();
            true
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AgendaMsg::ScheduleSuccess(events) => {
                self.events = events;
                true
            },
            AgendaMsg::ScheduleFailure(api_error) => {
                api_error.handle_api_error();
                match api_error {
                    ApiError::Known(error) if error.kind == "counter_too_low" => {
                        refresh_events(ctx.link().clone());
                    }
                    _ => {},
                }
                false
            },
            AgendaMsg::AnnouncementsSuccess(announcements) => {
                self.announcements = announcements;
                false // Don't think we should refresh display of the page because it would cause high inconvenience and frustration to the users
            },
            AgendaMsg::Previous => {
                let prev_week = NaiveDateTime::new(self.selected_day.naive_local(), NaiveTime::from_hms(0, 0, 0)) - chrono::Duration::days(7);
                if self.selected_day.weekday() != Weekday::Mon {
                    self.selected_day = self.selected_day.pred();
                } else if self.selected_day.weekday() == Weekday::Mon && !has_event_on_day(&self.events, prev_week, Weekday::Sat) {
                    self.selected_day = self.selected_day.pred().pred().pred();
                } else {
                    self.selected_day = self.selected_day.pred().pred();
                }
                self.slider.borrow_mut().set_offset(-20 * (self.selected_day.num_days_from_ce() - 730000));
                true
            },
            AgendaMsg::Next => {
                let now = NaiveDateTime::new(self.selected_day.naive_local(), NaiveTime::from_hms(0, 0, 0));
                if self.selected_day.weekday() == Weekday::Sat {
                    self.selected_day = self.selected_day.succ().succ();
                } else if self.selected_day.weekday() != Weekday::Fri {
                    self.selected_day = self.selected_day.succ();
                } else if self.selected_day.weekday() == Weekday::Fri && !has_event_on_day(&self.events, now, Weekday::Sat) {
                    self.selected_day = self.selected_day.succ().succ().succ();
                } else {
                    self.selected_day = self.selected_day.succ();
                }
                self.slider.borrow_mut().set_offset(-20 * (self.selected_day.num_days_from_ce() - 730000));
                
                true
            },
            AgendaMsg::Goto {day, month, year} => {
                self.selected_day = Paris.ymd(year, month, day);
                self.slider.borrow_mut().set_offset(-20 * (self.selected_day.num_days_from_ce() - 730000));
                true
            }
            AgendaMsg::Refresh => {
                let window = window();
                match Reflect::get(&window.doc(), &JsValue::from_str("reflectTheme")) {
                    Ok(reflect_theme) => {
                        let reflect_theme: Function = match reflect_theme.dyn_into(){
                            Ok(reflect_theme) => reflect_theme,
                            Err(e) => {
                                log!("Failed to convert reflect theme: {:?}", e);
                                return true;
                            }
                        };
                    
                        Reflect::apply(&reflect_theme, &window.doc(), &Array::new()).expect("Failed to call reflectTheme");
                    }
                    Err(_) => log!("reflectTheme not found")
                }
                true
            },
            AgendaMsg::CloseAnnouncement => update_close_announcement(self),
            AgendaMsg::SetSelectedEvent(event) => {
                if event.is_some() {
                    self.slider.borrow_mut().disable();
                } else {
                    self.slider.borrow_mut().enable();
                }

                self.selected_event = Rc::new(event);
                true
            },
            AgendaMsg::FetchColors(new_colors) => {
                crate::COLORS.update_colors(new_colors);
                true
            },
            AgendaMsg::PushColors() => {
                crate::COLORS.push_colors();
                false
            },
            AgendaMsg::ApiFailure(api_error) => {
                api_error.handle_api_error();
                false
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let mobile = crate::slider::width() <= 1000;
        
        // Go on the first day of the week
        let mut current_day = self.selected_day;
        match mobile {
            true => current_day = current_day.pred().pred(),
            false => for _ in 0..self.selected_day.weekday().num_days_from_monday() {
                current_day = current_day.pred();
            },
        };

        // Check if there is room for the announcement on mobile
        let announcement = self.displayed_announcement.as_ref();
        let mut show_mobile_announcement = mobile && announcement.is_some();
        if show_mobile_announcement {
            let announcement_start = current_day.succ().succ().and_hms(18,30,0).timestamp() as u64;
            let announcement_end = current_day.succ().succ().and_hms(20,0,0).timestamp() as u64;
            let announcement_range = announcement_start..=announcement_end;

            match self.events.binary_search_by_key(&announcement_start, |e| e.start_unixtime) { // Check if an event starts exactly at that time.
                Ok(_) => {
                    show_mobile_announcement = false;
                },
                Err(mut idx) => {
                    if let Some(e) = self.events.get(idx) {
                        if announcement_range.contains(&(e.start_unixtime)) { // Check if the next event starts in the range
                            show_mobile_announcement = false;
                        } else {
                            idx -= 1;
                            while let Some(e) = self.events.get(idx) { // Check if a few previous events end in the range
                                if announcement_range.contains(&(e.end_unixtime)) {
                                    show_mobile_announcement = false;
                                    break;
                                }
                                if e.end_unixtime < announcement_start - 6*3600 { // Avoid backtracking too much, 6h is enough
                                    break;
                                }
                                idx -= 1;
                            }
                        }
                    }
                },
            };
        }
        let agenda_class = if show_mobile_announcement { "show-announcement" } else { "" };
        let announcement = announcement.map(|a| view_announcement(a, ctx));

        // Build each day and put events in them
        let mut days = Vec::new();
        let mut day_names = Vec::new();
        for d in 0..6 {
            let mut events = Vec::new();

            // Iterate over events, starting from the first one that starts during the current day
            let day_start = current_day.and_hms(0,0,0).timestamp() as u64;
            let mut idx = match self.events.binary_search_by_key(&day_start, |e| e.start_unixtime) {
                Ok(idx) => idx,
                Err(idx) => idx,
            };
            while let Some(e) = self.events.get(idx) {
                if e.start_unixtime > day_start + 24*3600 {
                    break;
                }
                events.push(html!{
                    <EventComp
                        day_of_week={d}
                        event={e.clone()}
                        day_start={current_day.and_hms(0,0,0).timestamp() as u64}
                        agenda_link={ctx.link().clone()}
                        show_announcement={false}>
                    </EventComp>
                });
                idx += 1;
            }

            let day_style = match mobile {
                true => format!("position: absolute; left: {}%;", (current_day.num_days_from_ce()-730000) * 20),
                false => String::new(),
            };

            day_names.push(html! {
                <span id={if current_day == self.selected_day {"selected-day"} else {""}}>
                    { format_day(current_day.weekday(), current_day.day()) }
                </span>
            });
            days.push(html! {
                <div class="day" style={day_style}>
                    { events }
                </div>
            });

            current_day = current_day.succ();
        }

        let calendar = html! {
            <Calendar
                agenda_link={ctx.link().clone()}
                day={self.selected_day.day()}
                month={self.selected_day.month()}
                year={self.selected_day.year()} />
        };
        let popup = html! {
            <Popup
                event={self.selected_event.clone()}
                agenda_link={ctx.link().clone()} />
        };
        let day_container_style = if mobile {format!("position: relative; right: {}%", 100 * (self.selected_day.num_days_from_ce() - 730000))} else {String::new()};
        template_html!(
            "templates/agenda.html",
            onclick_settings = {ctx.props().app_link.callback(|_| AppMsg::SetPage(Page::Settings))},
            onclick_previous = {ctx.link().callback(|_| AgendaMsg::Previous)},
            onclick_next = {ctx.link().callback(|_| AgendaMsg::Next)},
            opt_announcement = announcement,
            ...
        )
    }
}