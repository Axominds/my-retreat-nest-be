use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct Pagination {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

impl Pagination {
    pub fn limit(&self) -> u64 {
        self.page_size.unwrap_or(10)
    }

    pub fn page(&self) -> u64 {
        self.page.unwrap_or(1)
    }

    pub fn offset(&self) -> u64 {
        let page: u64 = self.page();
        if page == 0 {
            return 0;
        }
        (page - 1) * self.limit()
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct PaginationMeta {
    total: u64,
    total_pages: u64,
    page_size: u64,
    page: u64,
}

impl PaginationMeta {
    pub fn build(total: u64, total_pages: u64, page_size: u64, page: u64) -> PaginationMeta {
        PaginationMeta {
            total: total,
            total_pages: total_pages,
            page_size: page_size,
            page: page,
        }
    }
}
