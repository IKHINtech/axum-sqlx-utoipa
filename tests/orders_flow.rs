use axum_ecommerce_api::{
    db::{create_orm_conn, run_migrations},
    dto::{
        cart::AddToCartRequest,
        orders::{CheckoutRequest, PayOrderRequest},
    },
    entity::{products::ActiveModel as ProductActive, users::ActiveModel as UserActive},
    middleware::auth::AuthUser,
    routes::admin::{LowStockQuery, UpdateOrderStatusRequest},
    routes::params::Pagination,
    services::{admin_service, cart_service, order_service},
    state::AppState,
};
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set, Statement};
use uuid::Uuid;

// Integration flow: user adds to cart -> checkout -> pay; admin updates status and sees low stock.
#[tokio::test]
async fn checkout_pay_and_admin_low_stock_flow() -> anyhow::Result<()> {
    // Allow skipping when no DB is configured in the environment.
    let database_url = match std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
    {
        Ok(url) => url,
        Err(_) => {
            eprintln!(
                "Skipping test: set TEST_DATABASE_URL or DATABASE_URL to run integration flow tests."
            );
            return Ok(());
        }
    };

    let state = setup_state(&database_url).await?;

    // Seed users
    let user_id = create_user(&state, "user", "user@example.com").await?;
    let admin_id = create_user(&state, "admin", "admin@example.com").await?;

    // Seed product with stock
    let product = ProductActive {
        id: Set(Uuid::new_v4()),
        name: Set("Test Widget".into()),
        description: Set(Some("A product for testing".into())),
        price: Set(1000),
        stock: Set(10),
        created_at: NotSet,
    }
    .insert(&state.orm)
    .await?;

    let auth_user = AuthUser {
        user_id,
        role: "user".into(),
    };
    let auth_admin = AuthUser {
        user_id: admin_id,
        role: "admin".into(),
    };

    // Add to cart
    cart_service::add_to_cart(
        &state,
        &auth_user,
        AddToCartRequest {
            product_id: product.id,
            quantity: 2,
        },
    )
    .await?;

    // Checkout
    let checkout_resp = order_service::checkout(
        &state,
        &auth_user,
        CheckoutRequest {
            address: "Somewhere".into(),
            payment_method: "cash".into(),
        },
    )
    .await?;
    let order = checkout_resp.data.unwrap().order;
    assert_eq!(order.total_amount, 2000);

    // Pay
    let pay_resp = order_service::pay_order(
        &state,
        &auth_user,
        order.id,
        PayOrderRequest {
            invoice_number: order.invoice_number.clone(),
        },
    )
    .await?;
    let paid_order = pay_resp.data.unwrap().order;
    assert_eq!(paid_order.status, "paid");

    // Admin updates status
    let updated = admin_service::update_order_status(
        &state,
        &auth_admin,
        order.id,
        UpdateOrderStatusRequest {
            status: "shipped".into(),
        },
    )
    .await?;
    assert_eq!(updated.data.unwrap().status, "shipped");

    // Low stock should include the product after stock decreased to 8
    let low = admin_service::list_low_stock(
        &state,
        &auth_admin,
        LowStockQuery {
            pagination: Pagination {
                page: Some(1),
                per_page: Some(20),
            },
            threshold: Some(10),
        },
    )
    .await?;
    assert!(
        low.data.unwrap().items.iter().any(|p| p.id == product.id),
        "expected product to appear in low-stock list"
    );

    Ok(())
}

async fn setup_state(database_url: &str) -> anyhow::Result<AppState> {
    let orm = create_orm_conn(database_url).await?;
    run_migrations(&orm).await?;

    // Clean tables between runs
    let backend = orm.get_database_backend();
    orm.execute(Statement::from_string(
        backend,
        "TRUNCATE TABLE order_items, orders, cart_items, favorites, audit_logs, products, users RESTART IDENTITY CASCADE",
    ))
    .await?;

    Ok(AppState { orm })
}

async fn create_user(state: &AppState, role: &str, email: &str) -> anyhow::Result<Uuid> {
    let user = UserActive {
        id: Set(Uuid::new_v4()),
        email: Set(email.to_string()),
        password_hash: Set("dummy".into()),
        role: Set(role.into()),
        created_at: NotSet,
    }
    .insert(&state.orm)
    .await?;

    Ok(user.id)
}
