use crate::prelude::*;

#[derive(Clone, Properties)]
pub struct CalendarProps {
    pub agenda_link: Scope<Agenda>,
    pub day: u32,
    pub month: u32,
    pub year: i32,
}

impl PartialEq for CalendarProps {
    fn eq(&self, _other: &Self) -> bool { true }
}

pub enum Msg {
    Next,
    Previous,
    Goto { day: u32, month: u32, year: i32 },
    TriggerFold,
}

pub struct Calendar {
    selected_day: u32,
    selected_month: u32,
    selected_year: i32,
    folded: bool,
    on_click: Closure<dyn FnMut(web_sys::MouseEvent)>,
}

impl Component for Calendar {
    type Message = Msg;
    type Properties = CalendarProps;

    fn create(ctx: &Context<Self>) -> Self {
        let doc = window().doc();
        let link = ctx.link().clone();
        let on_click = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let Some(calendar_el) = doc.get_element_by_id("calendar-content") else {return};
            let rect = calendar_el.get_bounding_client_rect();

            let (mx, my) = (event.client_x() as f64, event.client_y() as f64);
            let (rx, ry) = (rect.x(), rect.y());
            let (rw, rh) = (rect.width(), rect.height());

            // Check the click was inside the calendar
            if ((ry..ry+rh).contains(&my) && (rx..rx+rw).contains(&mx)) || (my <= ry) { return; }

            link.send_message(Msg::TriggerFold);
        }) as Box<dyn FnMut(_)>);
        Calendar {
            selected_day: ctx.props().day,
            selected_month: ctx.props().month,
            selected_year: ctx.props().year as i32,
            folded: true,
            on_click,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Next if !self.folded => {
                if self.selected_month == 12 {
                    self.selected_month = 1;
                    self.selected_year += 1;
                } else {
                    self.selected_month += 1;
                }
                self.selected_day = 1;
                ctx.props().agenda_link.send_message(AgendaMsg::Goto {
                    day: self.selected_day,
                    month: self.selected_month,
                    year: self.selected_year
                });
            },
            Msg::Previous if !self.folded => {
                if self.selected_month == 1 {
                    self.selected_month = 12;
                    self.selected_year -= 1;
                } else {
                    self.selected_month -= 1;
                }
                let last_day = NaiveDate::from_ymd(self.selected_year, (self.selected_month % 12) + 1, 1).pred();
                self.selected_day = last_day.day();
                ctx.props().agenda_link.send_message(AgendaMsg::Goto {
                    day: self.selected_day,
                    month: self.selected_month,
                    year: self.selected_year
                });
            },
            Msg::Next => {
                let next_week = NaiveDateTime::new(NaiveDate::from_ymd(self.selected_year, self.selected_month, self.selected_day), NaiveTime::from_hms(0, 0, 0)) + chrono::Duration::days(7);
                ctx.link().send_message(Msg::Goto {
                    day: next_week.day(),
                    month: next_week.month(),
                    year: next_week.year() as i32
                });
            },
            Msg::Previous => {
                let previous_week = NaiveDateTime::new(NaiveDate::from_ymd(self.selected_year, self.selected_month, self.selected_day), NaiveTime::from_hms(0, 0, 0)) - chrono::Duration::days(7);
                ctx.link().send_message(Msg::Goto {
                    day: previous_week.day(),
                    month: previous_week.month(),
                    year: previous_week.year() as i32
                });
            }
            Msg::Goto { day, month, year } => {
                self.selected_day = day;
                self.selected_month = month;
                self.selected_year = year;
                ctx.props().agenda_link.send_message(AgendaMsg::Goto {day, month,year});
            },
            Msg::TriggerFold => {
                self.folded = !self.folded;
                match self.folded {
                    true => window().remove_event_listener_with_callback("click", self.on_click.as_ref().unchecked_ref()).unwrap(),
                    false => window().add_event_listener_with_callback("click", self.on_click.as_ref().unchecked_ref()).unwrap(),
                };
            } 
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let display_month = format!("{} {}", t(match self.selected_month {
            1 => "Janvier",
            2 => "Février",
            3 => "Mars",
            4 => "Avril",
            5 => "Mai",
            6 => "Juin",
            7 => "Juillet",
            8 => "Août",
            9 => "Septembre",
            10 => "Octobre",
            11 => "Novembre",
            12 => "Décembre",
            _ => unreachable!(),
        }), self.selected_year);

        let first_day = NaiveDate::from_ymd(self.selected_year, self.selected_month, 1);
        let last_day = NaiveDate::from_ymd(self.selected_year, (self.selected_month % 12) + 1, 1).pred();

        let mut calendar_cases = Vec::new();
        for _ in 0..first_day.weekday().number_from_monday() - 1 {
            calendar_cases.push(html! {
                <span class="calendar-case" onclick={ctx.link().callback(|_| Msg::Previous)}></span>
            });
        }
        for day in 1..=last_day.day() {
            let month = self.selected_month;
            let year = self.selected_year;
            let date = NaiveDate::from_ymd(year, month, day);
            if date.weekday() == Weekday::Sun {
                let day_to_go = day - 1;
                calendar_cases.push(html! {
                    <span class="calendar-case disabled" id={if day==self.selected_day {Some("selected-calendar-case")} else {None}} onclick={ctx.link().callback(move |_| Msg::Goto {day: day_to_go,month,year})}>{day.to_string()}</span>
                });
            } else {
                calendar_cases.push(html! {
                    <span class="calendar-case" id={if day==self.selected_day {Some("selected-calendar-case")} else {None}} onclick={ctx.link().callback(move |_| Msg::Goto {day,month,year})}>{day.to_string()}</span>
                });
            }
            
        }
        while calendar_cases.len() % 7 != 0 {
            calendar_cases.push(html! {
                <span class="calendar-case" onclick={ctx.link().callback(|_| Msg::Next)}></span>
            });
        }

        let mut week_iter = Vec::new();
        let mut cases_iter = Vec::new();
        for week in 1..=calendar_cases.len()/7 {
            week_iter.push(week);
            cases_iter.push(calendar_cases.drain(0..7).collect::<Vec<_>>());
        }

        template_html! {
            "templates/components/calendar.html",
            onclick_previous = {ctx.link().callback(|_| Msg::Previous)},
            onclick_fold = {ctx.link().callback(|_| Msg::TriggerFold)},
            onclick_next = {ctx.link().callback(|_| Msg::Next)},
            week_iter = {week_iter.into_iter()},
            cases_iter = {cases_iter.into_iter()},
            is_folded = {self.folded},
        }
    }
}
