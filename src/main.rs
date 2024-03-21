#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use may_minihttp::HttpServiceFactory;
use may_todos::dispatcher::DispatcherService;
use may_todos::pool::PgConnectionPool;

struct HttpServer {
    pool: PgConnectionPool,
}

impl HttpServiceFactory for HttpServer {
    type Service = DispatcherService;

    fn new_service(&self, id: usize) -> Self::Service {
        let db = self.pool.get_connection(id);
        DispatcherService::new(db)
    }
}

fn main() {
    // TODO: read config from env vars
    may::config().set_pool_capacity(1000).set_stack_size(0x1000);
    println!("Starting http server on port 8080");
    let server = HttpServer {
        pool: PgConnectionPool::new(
            "postgres://postgres:password1@127.0.0.1:5432/may2",
            num_cpus::get(),
        ),
    };
    server.start("0.0.0.0:8080").unwrap().join().unwrap();
}
