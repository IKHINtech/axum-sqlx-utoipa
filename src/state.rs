use crate::db::{DbPool, OrmConn};

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub orm: OrmConn,
}
