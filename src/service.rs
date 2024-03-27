use crate::conn::PgConnection;

use atoi::atoi;
use matchit::Params;
use may_minihttp::Response;
use yarte::Serialize;

// Name length range
const MAX_CHARS: usize = 100;
const MIN_CHARS: usize = 3;

pub struct Service {
    pub db: PgConnection,
}

impl Service {
    #[inline]
    pub fn get_stories(&self, rsp: &mut Response) {
        let stories = self.db.get_stories().unwrap_or_default();
        rsp.header("Content-Type: application/json");
        stories.to_bytes_mut(rsp.body_mut());
    }

    #[inline]
    pub fn get_story(&self, params: Params, rsp: &mut Response) {
        let param = params.get("id").unwrap_or_default();
        let id = atoi::<i32>(param.as_bytes()).unwrap_or_default();
        if id <= 0 {
            rsp.status_code(400, "");
            return;
        }
        if let Ok(story) = self.db.get_story(id) {
            rsp.header("Content-Type: application/json");
            story.to_bytes_mut(rsp.body_mut());
        } else {
            rsp.status_code(404, "");
        }
    }

    #[inline]
    pub fn create_story(&self, body: &[u8], rsp: &mut Response) {
        let value: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
        if !value.is_object() {
            rsp.status_code(400, "");
            return;
        }

        let name = value
            .as_object()
            .map(|o| o.get("name").and_then(|o| o.as_str()).unwrap_or_default())
            .unwrap_or_default()
            .trim();

        let num_chars = name.chars().count();
        if num_chars < MIN_CHARS || num_chars > MAX_CHARS {
            rsp.status_code(400, "");
            return;
        }

        if let Ok(story) = self.db.create_story(name) {
            rsp.status_code(201, "");
            rsp.header("Content-Type: application/json");
            story.to_bytes_mut(rsp.body_mut());
        } else {
            rsp.status_code(500, "");
        }
    }

    #[inline]
    pub fn delete_story(&self, params: Params, rsp: &mut Response) {
        let param = params.get("id").unwrap_or_default();
        let id = atoi::<i32>(param.as_bytes()).unwrap_or_default();
        if id <= 0 {
            rsp.status_code(400, "");
            return;
        }
        if let Ok(rows) = self.db.delete_story(id) {
            let status = if rows > 0 { 204 } else { 404 };
            rsp.status_code(status, "");
        } else {
            rsp.status_code(500, "");
        }
    }
}
