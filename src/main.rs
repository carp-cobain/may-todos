#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use atoi::atoi;
use matchit::{Match, Params, Router};
use smallvec::SmallVec;
use std::io::{self, Read};
use std::sync::Arc;
use yarte::Serialize;

use may_minihttp::{HttpService, HttpServiceFactory, Request, Response};
use may_postgres::{self, Client, Statement};

// Constants ------------------------------------------------------------------

// SQL queries
const SQL_SELECT_STORIES: &str = "select * from stories limit 32";
const SQL_SELECT_STORY: &str = "select * from stories where id = $1";
const SQL_INSERT_STORY: &str = "insert into stories (name) values ($1) returning id";

// Error codes
const ERROR_SQL_QUERY: u8 = 1;
const ERROR_NOT_FOUND: u8 = 2;

// Dispatch codes
const STORIES: u8 = 1;
const STORY: u8 = 2;

// Domain ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct Story {
    id: i32,
    name: String,
}

// Database -------------------------------------------------------------------

struct PgStatements {
    select_stories: Statement,
    select_story: Statement,
    insert_story: Statement,
}

struct PgConnection {
    client: Client,
    statements: Arc<PgStatements>,
}

impl PgConnection {
    fn new(db_url: &str) -> Self {
        let client = may_postgres::connect(db_url).unwrap();

        let select_stories = client.prepare(SQL_SELECT_STORIES).unwrap();
        let select_story = client.prepare(SQL_SELECT_STORY).unwrap();
        let insert_story = client.prepare(SQL_INSERT_STORY).unwrap();

        let statements = Arc::new(PgStatements {
            select_stories,
            select_story,
            insert_story,
        });

        Self { client, statements }
    }

    fn get_stories(&self) -> Result<SmallVec<[Story; 32]>, u8> {
        let mut stories = SmallVec::<[_; 32]>::new();

        let results = self
            .client
            .query_raw(&self.statements.select_stories, &[])
            .map_err(|_| ERROR_SQL_QUERY)?;

        for result in results {
            let row = result.map_err(|_| ERROR_SQL_QUERY)?;
            stories.push(Story {
                id: row.get(0),
                name: row.get(1),
            })
        }

        Ok(stories)
    }

    fn get_story(&self, id: i32) -> Result<Story, u8> {
        let mut q = self
            .client
            .query_raw(&self.statements.select_story, &[&id])
            .map_err(|_| ERROR_SQL_QUERY)?;

        if let Some(result) = q.next() {
            let row = result.map_err(|_| ERROR_SQL_QUERY)?;
            let story = Story {
                id: row.get(0),
                name: row.get(1),
            };
            Ok(story)
        } else {
            Err(ERROR_NOT_FOUND)
        }
    }

    fn create_story(&self, name: &str) -> Result<Story, u8> {
        let mut q = self
            .client
            .query_raw(&self.statements.insert_story, &[&name])
            .map_err(|_| ERROR_SQL_QUERY)?;

        if let Some(result) = q.next() {
            let row = result.map_err(|_| ERROR_SQL_QUERY)?;
            let id: i32 = row.get(0);
            Ok(Story {
                id,
                name: name.into(),
            })
        } else {
            Err(ERROR_NOT_FOUND)
        }
    }
}

struct PgConnectionPool {
    connections: Vec<PgConnection>,
}

impl PgConnectionPool {
    fn new(db_url: &'static str, size: usize) -> PgConnectionPool {
        let connections = (0..size)
            .map(|_| std::thread::spawn(move || PgConnection::new(db_url)))
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>();
        PgConnectionPool { connections }
    }

    fn get_connection(&self, id: usize) -> PgConnection {
        let len = self.connections.len();
        let connection = &self.connections[id % len];
        PgConnection {
            client: connection.client.clone(),
            statements: connection.statements.clone(),
        }
    }
}

// Service --------------------------------------------------------------------

struct TodoService {
    router: Router<u8>,
    db: PgConnection,
}

impl TodoService {
    fn new(db: PgConnection) -> Self {
        let mut router = Router::new();
        router.insert("/stories", STORIES).unwrap();
        router.insert("/stories/{id}", STORY).unwrap();
        Self { router, db }
    }

    #[inline(always)]
    fn dispatch(&self, route: Match<'_, '_, &u8>, method: &str, body: &[u8], rsp: &mut Response) {
        match (method, route.value) {
            ("GET", &STORIES) => self.get_stories(rsp),
            ("GET", &STORY) => self.get_story(route.params, rsp),
            ("POST", &STORIES) => self.create_story(body, rsp),
            _ => {
                rsp.status_code(404, "");
                ()
            }
        }
    }

    #[inline(always)]
    fn get_stories(&self, rsp: &mut Response) {
        let stories = self.db.get_stories().unwrap_or_default();
        rsp.header("Content-Type: application/json");
        stories.to_bytes_mut(rsp.body_mut());
    }

    #[inline(always)]
    fn get_story(&self, params: Params, rsp: &mut Response) {
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

    #[inline(always)]
    fn create_story(&self, body: &[u8], rsp: &mut Response) {
        let value: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
        if !value.is_object() {
            rsp.status_code(400, "");
            return;
        }

        let name = value
            .as_object()
            .map(|o| o.get("name"))
            .flatten()
            .map(|o| o.as_str())
            .flatten()
            .unwrap_or_default();

        if name.is_empty() {
            rsp.status_code(400, "");
            return;
        }

        if let Ok(story) = self.db.create_story(&name) {
            rsp.status_code(201, "");
            rsp.header("Content-Type: application/json");
            story.to_bytes_mut(rsp.body_mut());
        } else {
            rsp.status_code(400, "");
        }
    }
}

impl HttpService for TodoService {
    fn call(&mut self, req: Request, rsp: &mut Response) -> io::Result<()> {
        let path = req.path().to_owned();
        let method = req.method().to_owned();
        let mut body = req.body();

        let mut buf = String::new();
        if method == "POST" {
            if let Err(_) = body.read_to_string(&mut buf) {
                rsp.status_code(400, "");
                return Ok(());
            }
        }

        if let Ok(route) = self.router.at(&path) {
            self.dispatch(route, &method, buf.as_bytes(), rsp);
        } else {
            rsp.status_code(404, "");
        }

        Ok(())
    }
}

// HTTP Server ----------------------------------------------------------------

struct HttpServer {
    pool: PgConnectionPool,
}

impl HttpServiceFactory for HttpServer {
    type Service = TodoService;

    fn new_service(&self, id: usize) -> Self::Service {
        let db = self.pool.get_connection(id);
        TodoService::new(db)
    }
}

// Main -----------------------------------------------------------------------

fn main() {
    may::config().set_pool_capacity(1000).set_stack_size(0x1000);
    println!("Starting http server on port 8080");
    let server = HttpServer {
        pool: PgConnectionPool::new(
            "postgres://postgres:password1@127.0.0.1:5432/may",
            num_cpus::get(),
        ),
    };
    server.start("0.0.0.0:8080").unwrap().join().unwrap();
}
