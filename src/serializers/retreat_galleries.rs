use serde::Serialize;

use crate::{entities_helper::RetreatGalleriesModel, map_fields};

#[derive(Serialize, Clone, Debug)]
pub struct ReadRetreatGallerySerializer{
    gallery_id: i64,
    retreat_id: i64,
    caption: Option<String>,
    order: Option<i32>,
    gallery_category_id: i64,
    created_by: Option<i64>,
    updated_by: Option<i64>
}

impl From<RetreatGalleriesModel> for ReadRetreatGallerySerializer {
    fn from(value: RetreatGalleriesModel) -> Self {
        map_fields!(value, ReadRetreatGallerySerializer, {
            gallery_id,
            retreat_id,
            caption,
            order,
            gallery_category_id,
            created_by,
            updated_by
        })
    }
}