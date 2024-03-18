#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use smallvec::SmallVec;
use std::io;
use std::sync::Arc;
use yarte::Serialize;

use may_minihttp::{HttpService, HttpServiceFactory, Request, Response};
use may_postgres::{self, Client, Statement};

// Domain ---------------------------------------------------------------------

#[derive(Serialize)]
pub struct Story {
    id: i32,
    name: String,
}

// Database -------------------------------------------------------------------

struct PgStatements {
    select_stories: Statement,
}

struct PgConnection {
    client: Client,
    statements: Arc<PgStatements>,
}

impl PgConnection {
    fn new(db_url: &str) -> Self {
        let client = may_postgres::connect(db_url).unwrap();
        let select_stories = client.prepare("SELECT * FROM stories").unwrap();
        let statements = Arc::new(PgStatements { select_stories });
        Self { client, statements }
    }

    fn get_stories(&self) -> Result<SmallVec<[Story; 32]>, may_postgres::Error> {
        let mut stories = SmallVec::<[_; 32]>::new();
        for result in self
            .client
            .query_raw(&self.statements.select_stories, &[])?
        {
            let row = result?;
            stories.push(Story {
                id: row.get(0),
                name: row.get(1),
            })
        }
        Ok(stories)
    }
}

struct PgConnectionPool {
    connections: Vec<PgConnection>,
}

impl PgConnectionPool {
    fn new(db_url: &'static str, size: usize) -> PgConnectionPool {
        let connections = (0..size)
            .map(|_| std::thread::spawn(move || PgConnection::new(db_url)))
            .collect::<Vec<_>>();
        let mut connections: Vec<_> = connections.into_iter().map(|t| t.join().unwrap()).collect();
        connections.sort_by(|a, b| (a.client.id() % size).cmp(&(b.client.id() % size)));
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
    db: PgConnection,
}

impl HttpService for TodoService {
    fn call(&mut self, req: Request, rsp: &mut Response) -> io::Result<()> {
        match req.path() {
            "/may/api/v1/stories" => {
                rsp.header("Content-Type: application/json");
                let stories = self.db.get_stories().unwrap();
                stories.to_bytes_mut(rsp.body_mut());
            }
            _ => {
                rsp.status_code(404, "");
            }
        }
        Ok(())
    }
}

// HTTP Server ----------------------------------------------------------------

struct HttpServer {
    db_pool: PgConnectionPool,
}

impl HttpServiceFactory for HttpServer {
    type Service = TodoService;
    fn new_service(&self, id: usize) -> Self::Service {
        let db = self.db_pool.get_connection(id);
        TodoService { db }
    }
}

// Main -----------------------------------------------------------------------

fn main() {
    may::config().set_pool_capacity(1000).set_stack_size(0x1000);
    println!("Starting http server: 127.0.0.1:8080");
    let server = HttpServer {
        db_pool: PgConnectionPool::new(
            "postgres://postgres:password1@127.0.0.1:5432/may",
            num_cpus::get(),
        ),
    };
    server.start("0.0.0.0:8080").unwrap().join().unwrap();
}
