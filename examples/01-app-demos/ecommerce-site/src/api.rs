use dioxus::prelude::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

// Cache up to 100 requests, invalidating them after 60 seconds
pub(crate) async fn fetch_product(product_id: usize) -> Result<Product> {
    Ok(
        reqwest::get(format!("https://fakestoreapi.com/products/{product_id}"))
            .await?
            .json()
            .await?,
    )
}

// Cache up to 100 requests, invalidating them after 60 seconds
pub(crate) async fn fetch_products(count: usize, sort: Sort) -> Result<Vec<Product>> {
    Ok(reqwest::get(format!(
        "https://fakestoreapi.com/products/?sort={sort}&limit={count}"
    ))
    .await?
    .json()
    .await?)
}

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
