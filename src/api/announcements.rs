use super::*;
use crate::prelude::*;

fn load_cached_announcements() -> Option<(i64, Vec<AnnouncementDesc>)> {
    let local_storage = window().local_storage().unwrap().unwrap();

    let last_updated = match local_storage.get("last_updated_announcements").map(|v| v.map(|v| v.parse())) {
        Ok(Some(Ok(last_updated))) => last_updated,
        _ => return None,
    };
    
    let cached_announcements_str = match local_storage.get("cached_announcements") {
        Ok(Some(cached_announcements_str)) => cached_announcements_str,
        _ => return None,
    };

    let cached_announcements = match serde_json::from_str::<Vec<AnnouncementDesc>>(&cached_announcements_str) {
        Ok(cached_announcements) => cached_announcements,
        _ => return None,
    };

    Some((last_updated, cached_announcements))
}

async fn load_announcements() -> Result<Vec<AnnouncementDesc>, ApiError> {
    let (api_key, counter) = get_login_info();

    let request = Request::new_with_str("/api/announcements")?;

    request.headers().set(
        "Api-Key",
        &format!("{}-{}-{}", api_key, counter, gen_code(api_key, counter)),
    )?;

    let resp = JsFuture::from(window().fetch_with_request(&request)).await?;
    let resp: web_sys::Response = resp.dyn_into()?;
    let json = JsFuture::from(resp.json()?).await?;

    if resp.status() == 400 || resp.status() == 500 {
        let error: KnownApiError = match json.into_serde() {
            Ok(error) => error,
            _ => return Err(ApiError::Unknown(json)),
        };
        return Err(error.into());
    }

    let events: Vec<AnnouncementDesc> = match json.into_serde() {
        Ok(events) => events,
        _ => return Err(ApiError::Unknown(json)),
    };

    let now = (js_sys::Date::new_0().get_time() / 1000.0) as i64;
    let local_storage = window().local_storage().unwrap().unwrap();
    let _ = local_storage.set("last_updated_announcements", &now.to_string());
    let _ = local_storage.set("cached_announcements", &serde_json::to_string(&events).unwrap());

    Ok(events)
}

pub fn init_announcements(now: DateTime<chrono_tz::Tz>, agenda_link: Scope<Agenda>) -> Vec<AnnouncementDesc> {
    // Get cached
    let mut announcements = Vec::new();
    if let Some((last_updated, cached)) = load_cached_announcements() {
        announcements = cached;
        if last_updated > now.timestamp() - 3600*12 && !announcements.is_empty() {
            return announcements;
        }
    }

    // Update from server
    wasm_bindgen_futures::spawn_local(async move {
        match load_announcements().await {
            Ok(announcements) => agenda_link.send_message(AgendaMsg::AnnouncementsSuccess(announcements)),
            Err(e) => e.handle_api_error(),
        }
    });

    announcements
}
