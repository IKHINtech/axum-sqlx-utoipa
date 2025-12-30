use argon2::{
    Argon2, PasswordHasher,
    password_hash::{rand_core::OsRng, SaltString},
};
use axum_ecommerce_api::{
    config::AppConfig,
    db::create_pool,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;

    let pool = create_pool(&config.database_url).await?;
    // Ensure migrations are applied.
    sqlx::migrate!("./migrations").run(&pool).await?;

    let admin_id = ensure_admin(&pool, "admin@example.com", "admin123").await?;
    let user_id = ensure_user(&pool, "user@example.com", "user123").await?;
    seed_products(&pool).await?;

    println!("Seed completed. Admin ID: {admin_id}, User ID: {user_id}");
    Ok(())
}

async fn ensure_admin(pool: &sqlx::PgPool, email: &str, password: &str) -> anyhow::Result<Uuid> {
    ensure_user_with_role(pool, email, password, "admin").await
}

async fn ensure_user(pool: &sqlx::PgPool, email: &str, password: &str) -> anyhow::Result<Uuid> {
    ensure_user_with_role(pool, email, password, "user").await
}

async fn ensure_user_with_role(
    pool: &sqlx::PgPool,
    email: &str,
    password: &str,
    role: &str,
) -> anyhow::Result<Uuid> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .to_string();

    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        INSERT INTO users (id, email, password_hash, role)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (email) DO UPDATE SET role = EXCLUDED.role
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .fetch_optional(pool)
    .await?;

    // If user already exists, fetch id
    let user_id = match row {
        Some((id,)) => id,
        None => {
            let existing: (Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1")
                .bind(email)
                .fetch_one(pool)
                .await?;
            existing.0
        }
    };

    println!("Ensured user {email} (role={role})");
    Ok(user_id)
}

async fn seed_products(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let products = vec![
        ("Axum Hoodie", "Warm hoodie for Rustaceans", 550000, 50),
        ("Ferris Mug", "Coffee tastes better with Ferris", 120000, 100),
        ("Rust Sticker Pack", "Decorate your laptop", 50000, 200),
        ("E-book: Async Rust", "Learn async Rust patterns", 250000, 75),
    ];

    for (name, desc, price, stock) in products {
        sqlx::query(
            r#"
            INSERT INTO products (id, name, description, price, stock)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (name) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(desc)
        .bind(price)
        .bind(stock)
        .execute(pool)
        .await?;
    }

    println!("Seeded products");
    Ok(())
}
