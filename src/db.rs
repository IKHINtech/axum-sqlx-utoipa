use anyhow::Result;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};
use std::path::PathBuf;
use tokio::fs;

/// Create a SeaORM connection.
pub async fn create_orm_conn(database_url: &str) -> Result<DatabaseConnection> {
    let conn = Database::connect(database_url).await?;
    Ok(conn)
}

/// Minimal migration runner that executes SQL files in `migrations/` in filename order.
pub async fn run_migrations(conn: &DatabaseConnection) -> Result<()> {
    let mut entries = fs::read_dir("migrations").await?;
    let mut files: Vec<PathBuf> = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    files.sort();

    let backend = conn.get_database_backend();
    for file in files {
        let sql = fs::read_to_string(&file).await?;
        // Postgres prepared statements cannot contain multiple commands,
        // so split the migration file and run each statement individually.
        for stmt in sql.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            let statement = format!("{stmt};");
            conn.execute(Statement::from_string(backend, statement))
                .await?;
        }
    }

    Ok(())
}
