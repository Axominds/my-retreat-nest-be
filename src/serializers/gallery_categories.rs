use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{entities_helper::GalleryCategoriesModel, map_fields};

#[derive(Deserialize, Clone, Debug, Validate)]
pub struct CreateGalleryCategorySerializer{
    pub name: String
}


#[derive(Deserialize, Clone, Debug, Validate)]
pub struct UpdateGalleryCategorySerializer{
    pub name: Option<String>
}

#[derive(Serialize, Clone, Debug, Validate)]
pub struct ReadGalleryCategorySerializer{
    pub gallery_category_id: i64,
    pub name: String,
    pub retreat_id: i64,
}

impl From<GalleryCategoriesModel> for ReadGalleryCategorySerializer{
    fn from(value: GalleryCategoriesModel) -> Self {
        map_fields!(value, ReadGalleryCategorySerializer, {
            gallery_category_id,
            name,
            retreat_id,
        })
    }

}
