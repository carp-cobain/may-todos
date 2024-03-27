#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use dotenv::dotenv;
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
    dotenv().ok();
    env_logger::init();

    let config = Config::default();
    let db_url = config.db_connection_string();
    let pool_size = num_cpus::get().max(4);
    let pool = PgConnectionPool::new(&db_url, pool_size);

    log::info!("Starting http server on {}", &config.listen_addr);
    HttpServer { pool }
        .start(config.listen_addr)
        .expect("failed to start server")
        .join()
        .unwrap();
}
