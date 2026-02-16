use actix_web::HttpResponse;
use serde::Serialize;

use crate::*;

#[derive(Serialize)]
pub struct SubsonicResponse<T> {
    #[serde(rename = "subsonic-response")]
    response: ResponseBody<T>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Failed,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody<T> {
    status: Status,
    version: &'static str,
    r#type: &'static str,
    server_version: &'static str,
    open_subsonic: bool,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    data: Option<T>,
}

impl<T: Serialize> ResponseBody<T> {
    pub const fn new(status: Status, data: Option<T>) -> Self {
        Self {
            status,
            version: API_VERSION,
            r#type: env!("CARGO_PKG_NAME"),
            server_version: "0.1.3 (tag)",
            open_subsonic: true,
            data,
        }
    }

    pub const fn ok() -> Self {
        Self::new(Status::Ok, None)
    }

    pub const fn ok_with(data: T) -> Self {
        Self::new(Status::Ok, Some(data))
    }

    pub const fn failed() -> Self {
        Self::new(Status::Failed, None)
    }

    // pub const fn failed_with(data: T) -> Self {
    //     Self::new(Status::Failed, Some(data))
    // }

    pub fn into_response(self) -> HttpResponse {
        let mut res = match self.status {
            Status::Ok => HttpResponse::Ok(),
            Status::Failed => HttpResponse::InternalServerError(),
        };
        res.json(SubsonicResponse { response: self })
    }
}
