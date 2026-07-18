use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Pagination {
	#[serde(default = "default_page")] // absent 1
	pub page: u32,
	#[serde(default = "default_limit")] // absent 20
	pub limit: u32,
}
#[derive(Serialize)]
pub struct PaginatedResponse<T>{
	pub data: Vec<T>,
	pub total: i64,
	pub page: u32,
	pub limit: u32,
	pub total_pages: u32,
}

fn default_page() -> u32 { 1 }
fn default_limit() -> u32 { 20 }