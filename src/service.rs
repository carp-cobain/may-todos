use crate::conn::PgConnection;

use atoi::atoi;
use matchit::Params;
use may_minihttp::Response;
use yarte::Serialize;

// Max name len
const MAX_LEN: usize = 100;

pub struct Service {
    pub db: PgConnection,
}

impl Service {
    pub fn get_stories(&self, rsp: &mut Response) {
        let stories = self.db.get_stories().unwrap_or_default();
        rsp.header("Content-Type: application/json");
        stories.to_bytes_mut(rsp.body_mut());
    }

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

        if name.is_empty() || name.len() > MAX_LEN {
            rsp.status_code(400, "");
            return;
        }

        if let Ok(story) = self.db.create_story(name) {
            rsp.status_code(201, "");
            rsp.header("Content-Type: application/json");
            story.to_bytes_mut(rsp.body_mut());
        } else {
            rsp.status_code(400, "");
        }
    }

    pub fn delete_story(&self, params: Params, rsp: &mut Response) {
        let param = params.get("id").unwrap_or_default();
        let id = atoi::<i32>(param.as_bytes()).unwrap_or_default();
        if id <= 0 {
            rsp.status_code(400, "");
            return;
        }
        if let Ok(rows) = self.db.delete_story(id) {
            if rows > 0 {
                rsp.status_code(204, "");
            } else {
                rsp.status_code(404, "");
            }
        } else {
            rsp.status_code(400, "");
        }
    }
}
