use {
    crate::models::historical_data::{Unit, validate_interval},
    serde::Serialize,
    serde_valid::{Validate, validation::Error},
};

fn validate_unit(unit: &Unit) -> Result<(), Error> {
    match unit {
        Unit::Minutes | Unit::Hours | Unit::Days => Ok(()),
        _ => Err(Error::Custom(
            "Unit must be minutes, hours or days".to_string(),
        )),
    }
}

#[derive(Serialize, Debug, Validate)]
#[validate(custom = |s| validate_interval(&s.unit, &s.interval))]
pub struct IntradayCandleDataV3Request {
    #[validate(
        pattern = r"^(?:^NSE_EQ|NSE_FO|NCD_FO|BSE_EQ|BSE_FO|BCD_FO|MCX_FO|NSE_INDEX|BSE_INDEX|MCX_INDEX)\|[\w ]+(,(?:NSE_EQ|NSE_FO|NCD_FO|BSE_EQ|BSE_FO|BCD_FO|MCX_FO|NSE_INDEX|BSE_INDEX|MCX_INDEX)\|[\w ]+)*?$",
        message = "Invalid instrument_key"
    )]
    pub instrument_key: String,
    #[validate(custom = validate_unit)]
    pub unit: Unit,
    pub interval: String,
}
