use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl Pagination {
    pub fn normalize(&self) -> (i64, i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;
        (page, per_page, offset)
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn as_sql(&self) -> &'static str {
        match self {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProductSortBy {
    CreatedAt,
    Price,
    Name,
}

impl ProductSortBy {
    pub fn as_sql(&self) -> &'static str {
        match self {
            ProductSortBy::CreatedAt => "created_at",
            ProductSortBy::Price => "price",
            ProductSortBy::Name => "name",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProductQuery {
    #[serde(flatten)]
    pub pagination: Pagination,
    pub q: Option<String>,
    pub min_price: Option<i64>,
    pub max_price: Option<i64>,
    pub sort_by: Option<ProductSortBy>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct OrderListQuery {
    #[serde(flatten)]
    pub pagination: Pagination,
    pub status: Option<String>,
    pub sort_order: Option<SortOrder>,
}
