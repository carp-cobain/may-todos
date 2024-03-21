use crate::domain::Story;
use may_postgres::{self, Client, Statement};
use smallvec::SmallVec;
use std::sync::Arc;

// SQL queries
const SQL_SELECT_STORIES: &str = "select * from stories order by id desc limit 32";
const SQL_SELECT_STORY: &str = "select * from stories where id = $1";
const SQL_INSERT_STORY: &str = "insert into stories (name) values ($1) returning id";
const SQL_DELETE_STORY: &str = "delete from stories where id = $1";

// Error codes
// TODO: Check performance of using errors
pub const ERROR_QUERY: u8 = 1;
pub const ERROR_NOT_FOUND: u8 = 2;
pub const ERROR_INSERT: u8 = 3;
pub const ERROR_DELETE: u8 = 4;

pub struct PgStatements {
    select_stories: Statement,
    select_story: Statement,
    insert_story: Statement,
    delete_story: Statement,
}

pub struct PgConnection {
    pub client: Client,
    pub statements: Arc<PgStatements>,
}

impl PgConnection {
    pub fn new(db_url: &str) -> Self {
        let client = may_postgres::connect(db_url).unwrap();
        let prepare = |sql| client.prepare(sql).unwrap();

        let statements = Arc::new(PgStatements {
            select_stories: prepare(SQL_SELECT_STORIES),
            select_story: prepare(SQL_SELECT_STORY),
            insert_story: prepare(SQL_INSERT_STORY),
            delete_story: prepare(SQL_DELETE_STORY),
        });

        Self { client, statements }
    }

    pub fn get_stories(&self) -> Result<SmallVec<[Story; 32]>, u8> {
        let mut stories = SmallVec::<[_; 32]>::new();

        let stream = self
            .client
            .query_raw(&self.statements.select_stories, &[])
            .map_err(|_| ERROR_QUERY)?;

        for result in stream {
            let row = result.map_err(|_| ERROR_QUERY)?;
            stories.push(Story {
                id: row.get(0),
                name: row.get(1),
            })
        }

        Ok(stories)
    }

    pub fn get_story(&self, id: i32) -> Result<Story, u8> {
        let mut stream = self
            .client
            .query_raw(&self.statements.select_story, &[&id])
            .map_err(|_| ERROR_QUERY)?;

        if let Some(result) = stream.next() {
            let row = result.map_err(|_| ERROR_QUERY)?;
            let name = row.get(1);
            Ok(Story { id, name })
        } else {
            Err(ERROR_NOT_FOUND)
        }
    }

    pub fn create_story(&self, name: &str) -> Result<Story, u8> {
        let mut stream = self
            .client
            .query_raw(&self.statements.insert_story, &[&name])
            .map_err(|_| ERROR_INSERT)?;

        if let Some(result) = stream.next() {
            let row = result.map_err(|_| ERROR_INSERT)?;
            let id: i32 = row.get(0);
            let name = String::from(name);
            Ok(Story { id, name })
        } else {
            Err(ERROR_INSERT)
        }
    }

    pub fn delete_story(&self, id: i32) -> Result<u64, u8> {
        self.client
            .execute_raw(&self.statements.delete_story, &[&id])
            .map_err(|_| ERROR_DELETE)
    }
}
