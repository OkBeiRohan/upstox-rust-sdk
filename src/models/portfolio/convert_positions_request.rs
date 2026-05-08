use {
    crate::models::{ProductType, TransactionType},
    serde::Serialize,
    serde_valid::{Validate, validation},
};

#[derive(Serialize, Debug, Validate)]
pub struct ConvertPositionsRequest {
    #[validate(
        pattern = r"^(?:^NSE_EQ|NSE_FO|NCD_FO|BSE_EQ|BSE_FO|BCD_FO|MCX_FO|NSE_INDEX|BSE_INDEX|MCX_INDEX)\|[\w ]+(,(?:NSE_EQ|NSE_FO|NCD_FO|BSE_EQ|BSE_FO|BCD_FO|MCX_FO|NSE_INDEX|BSE_INDEX|MCX_INDEX)\|[\w ]+)*?$",
        message = "Invalid instrument_token"
    )]
    pub instrument_token: String,
    #[validate(custom = |p| product_validation(p, "new_product"))]
    pub new_product: ProductType,
    #[validate(custom = |p| product_validation(p, "old_product"))]
    pub old_product: ProductType,
    pub transaction_type: TransactionType,
    #[validate(exclusive_minimum = 0, message = "quantity must be greater than 0")]
    pub quantity: u32,
}

fn product_validation(product: &ProductType, field: &str) -> Result<(), validation::Error> {
    match product != &ProductType::CO {
        true => Ok(()),
        false => Err(validation::Error::Custom(format!("{} cannot be CO", field))),
    }
}
