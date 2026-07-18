use serde::{Deserialize, Serialize};

pub trait Paginate {
    fn limit(&self) -> u64;
    fn page(&self) -> u64;
    fn offset(&self) -> u64;
    fn build_meta(&self, total: u64) -> PaginationMeta {
        let page_size = self.limit();
        let page = self.page();
        let total_pages = (total + page_size - 1) / page_size;
        PaginationMeta::build(total, total_pages, page_size, page)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Pagination {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

impl Paginate for Pagination {
    fn limit(&self) -> u64 {
        self.page_size.unwrap_or(10)
    }

    fn page(&self) -> u64 {
        self.page.unwrap_or(1)
    }

    fn offset(&self) -> u64 {
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
            total,
            total_pages,
            page_size,
            page,
        }
    }
}
