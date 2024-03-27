use crate::conn::PgConnection;

pub struct PgConnectionPool {
    pub connections: Vec<PgConnection>,
}

impl PgConnectionPool {
    pub fn new(db_url: &str, size: usize) -> PgConnectionPool {
        PgConnectionPool {
            connections: (0..size)
                .map(|_| PgConnection::new(db_url))
                .collect::<Vec<_>>(),
        }
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
