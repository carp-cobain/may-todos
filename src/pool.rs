use crate::conn::PgConnection;

pub struct PgConnectionPool {
    pub connections: Vec<PgConnection>,
}

impl PgConnectionPool {
    pub fn new(db_url: &'static str, size: usize) -> PgConnectionPool {
        let connections = (0..size)
            .map(|_| std::thread::spawn(move || PgConnection::new(db_url)))
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>();
        PgConnectionPool { connections }
    }

    pub fn get_connection(&self, id: usize) -> PgConnection {
        let len = self.connections.len();
        let connection = &self.connections[id % len];
        PgConnection {
            client: connection.client.clone(),
            statements: connection.statements.clone(),
        }
    }
}
