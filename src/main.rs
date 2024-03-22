#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use may_minihttp::HttpServiceFactory;
use may_todos::{config::Config, dispatcher::DispatcherService, pool::PgConnectionPool};

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
    may::config().set_pool_capacity(1000).set_stack_size(0x1000);

    let config = Config::default();
    let db_url = config.db_connection_string();
    let pool = PgConnectionPool::new(&db_url, num_cpus::get());

    println!("Starting http server on {}", &config.listen_addr);
    let server = HttpServer { pool };
    server.start(config.listen_addr).unwrap().join().unwrap();
}
