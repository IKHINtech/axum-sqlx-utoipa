use argon2::{
    Argon2, PasswordHasher,
    password_hash::{rand_core::OsRng, SaltString},
};
use axum_ecommerce_api::{
    config::AppConfig,
    db::{create_orm_conn, run_migrations},
    entity::{
        products::{ActiveModel as ProductActive, Column as ProdCol, Entity as Products},
        users::{ActiveModel as UserActive, Column as UserCol, Entity as Users, Model as UserModel},
    },
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sea_orm::ActiveValue::NotSet;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;

    let orm = create_orm_conn(&config.database_url).await?;
    // Ensure migrations are applied.
    run_migrations(&orm).await?;

    let admin_id = ensure_admin(&orm, "admin@example.com", "admin123").await?;
    let user_id = ensure_user(&orm, "user@example.com", "user123").await?;
    seed_products(&orm).await?;

    println!("Seed completed. Admin ID: {admin_id}, User ID: {user_id}");
    Ok(())
}

async fn ensure_admin(orm: &sea_orm::DatabaseConnection, email: &str, password: &str) -> anyhow::Result<Uuid> {
    ensure_user_with_role(orm, email, password, "admin").await
}

async fn ensure_user(orm: &sea_orm::DatabaseConnection, email: &str, password: &str) -> anyhow::Result<Uuid> {
    ensure_user_with_role(orm, email, password, "user").await
}

async fn ensure_user_with_role(
    orm: &sea_orm::DatabaseConnection,
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

    if let Some(existing) = Users::find()
        .filter(UserCol::Email.eq(email.to_string()))
        .one(orm)
        .await? {
        if existing.role != role {
            let mut active: UserActive = existing.clone().into();
            active.role = Set(role.to_string());
            active.update(orm).await?;
        }
        println!("Ensured user {email} (role={role})");
        return Ok(existing.id);
    }

    let active = UserActive {
        id: Set(Uuid::new_v4()),
        email: Set(email.to_string()),
        password_hash: Set(password_hash),
        role: Set(role.to_string()),
        created_at: NotSet,
    };

    let model: UserModel = active.insert(orm).await?;

    println!("Ensured user {email} (role={role})");
    Ok(model.id)
}

async fn seed_products(orm: &sea_orm::DatabaseConnection) -> anyhow::Result<()> {
    let products = vec![
        ("Axum Hoodie", "Warm hoodie for Rustaceans", 550000, 50),
        ("Ferris Mug", "Coffee tastes better with Ferris", 120000, 100),
        ("Rust Sticker Pack", "Decorate your laptop", 50000, 200),
        ("E-book: Async Rust", "Learn async Rust patterns", 250000, 75),
    ];

    for (name, desc, price, stock) in products {
        let existing = Products::find().filter(ProdCol::Name.eq(name)).one(orm).await?;
        if existing.is_some() {
            continue;
        }
        let active = ProductActive {
            id: Set(Uuid::new_v4()),
            name: Set(name.to_string()),
            description: Set(Some(desc.to_string())),
            price: Set(price),
            stock: Set(stock),
            created_at: NotSet,
        };
        active.insert(orm).await?;
    }

    println!("Seeded products");
    Ok(())
}
