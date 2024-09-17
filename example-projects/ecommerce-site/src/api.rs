#![allow(unused)]

use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub(crate) struct Product {
    pub(crate) id: u32,
    pub(crate) title: String,
    pub(crate) price: f32,
    pub(crate) description: String,
    pub(crate) category: String,
    pub(crate) image: String,
    pub(crate) rating: Rating,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub(crate) struct Rating {
    pub(crate) rate: f32,
    pub(crate) count: u32,
}

impl Display for Rating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rounded = self.rate.round() as usize;
        for _ in 0..rounded {
            "★".fmt(f)?;
        }
        for _ in 0..(5 - rounded) {
            "☆".fmt(f)?;
        }

        write!(f, " ({:01}) ({} ratings)", self.rate, self.count)?;

        Ok(())
    }
}

#[allow(unused)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd)]
pub(crate) enum Sort {
    Descending,
    Ascending,
}

impl Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sort::Descending => write!(f, "desc"),
            Sort::Ascending => write!(f, "asc"),
        }
    }
}

// Cache up to 100 requests, invalidating them after 60 seconds
pub(crate) async fn fetch_user_carts(user_id: usize) -> Result<Vec<Cart>, reqwest::Error> {
    reqwest::get(format!(
        "https://fakestoreapi.com/carts/user/{user_id}?startdate=2019-12-10&enddate=2023-01-01"
    ))
    .await?
    .json()
    .await
}

// Cache up to 100 requests, invalidating them after 60 seconds
pub(crate) async fn fetch_user(user_id: usize) -> dioxus::Result<Product> {
    Ok(
        reqwest::get(format!("https://fakestoreapi.com/users/{user_id}"))
            .await?
            .json()
            .await?,
    )
}

// Cache up to 100 requests, invalidating them after 60 seconds
pub(crate) async fn fetch_product(product_id: usize) -> dioxus::Result<Product> {
    Ok(
        reqwest::get(format!("https://fakestoreapi.com/products/{product_id}"))
            .await?
            .json()
            .await?,
    )
}

// Cache up to 100 requests, invalidating them after 60 seconds
pub(crate) async fn fetch_products(count: usize, sort: Sort) -> dioxus::Result<Vec<Product>> {
    Ok(reqwest::get(format!(
        "https://fakestoreapi.com/products/?sort={sort}&limit={count}"
    ))
    .await?
    .json()
    .await?)
}

#[derive(Serialize, Deserialize)]
pub(crate) struct User {
    id: usize,
    email: String,
    username: String,
    password: String,
    name: FullName,
    phone: String,
}

impl User {
    async fn fetch_most_recent_cart(&self) -> Result<Option<Cart>, reqwest::Error> {
        let all_carts = fetch_user_carts(self.id).await?;

        Ok(all_carts.into_iter().max_by_key(|cart| cart.date))
    }
}

#[derive(Serialize, Deserialize)]
struct FullName {
    firstname: String,
    lastname: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Cart {
    id: usize,
    #[serde(rename = "userId")]
    user_id: usize,
    data: String,
    products: Vec<ProductInCart>,
    date: DateTime<Utc>,
}

impl Cart {
    async fn update_database(&mut self) -> Result<(), reqwest::Error> {
        let id = self.id;
        let client = reqwest::Client::new();
        *self = client
            .put(format!("https://fakestoreapi.com/carts/{id}"))
            .send()
            .await?
            .json()
            .await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ProductInCart {
    #[serde(rename = "productId")]
    product_id: usize,
    quantity: usize,
}

impl ProductInCart {
    pub async fn fetch_product(&self) -> Result<Product, dioxus::CapturedError> {
        fetch_product(self.product_id).await
    }
}
