use crate::conn::PgConnection;
use crate::service::Service;

use matchit::{Match, Router};
use may_minihttp::{HttpService, Request, Response};
use std::io::{Read, Result};

// Router dispatch codes
const STORIES: u8 = 128;
const STORY: u8 = 129;

/// Maps routes to service methods
pub struct DispatcherService {
    router: Router<u8>,
    service: Service,
}

impl DispatcherService {
    /// Create a new dispatcher
    pub fn new(db: PgConnection) -> Self {
        let mut router = Router::new();
        router.insert("/stories", STORIES).unwrap();
        router.insert("/stories/{id}", STORY).unwrap();
        Self {
            router,
            service: Service { db },
        }
    }

    /// Dispatch requests to matched service methods.
    #[inline]
    fn dispatch(&self, route: Match<'_, '_, &u8>, method: &str, body: &[u8], rsp: &mut Response) {
        match (method, route.value) {
            ("POST", &STORIES) => self.service.create_story(body, rsp),
            ("GET", &STORIES) => self.service.get_stories(rsp),
            ("GET", &STORY) => self.service.get_story(route.params, rsp),
            ("DELETE", &STORY) => self.service.delete_story(route.params, rsp),
            _ => {
                rsp.status_code(404, "");
            }
        }
    }
}

impl HttpService for DispatcherService {
    fn call(&mut self, req: Request, rsp: &mut Response) -> Result<()> {
        if let Ok(route) = self.router.at(&req.path().to_owned()) {
            let method = req.method().to_owned();
            let mut body = req.body();
            let mut buf = Vec::new();
            if method == "POST" && body.read_to_end(&mut buf).is_err() {
                rsp.status_code(500, "");
                return Ok(());
            }
            self.dispatch(route, &method, &buf, rsp);
        } else {
            rsp.status_code(404, "");
        }

        Ok(())
    }
}
