use {
    crate::models::{SegmentType, trade_profit_and_loss::segment_validation},
    serde::Serialize,
    serde_valid::Validate,
};

#[derive(Serialize, Debug, Validate)]
pub struct PnLReportMetaDataRequest {
    #[validate(
        pattern = r"^(0[1-9]|[12][0-9]|3[01])\-(0[1-9]|1[012])\-\d{4}$",
        message = "from_date format must be dd-mm-yyyy"
    )]
    pub from_date: Option<String>,
    #[validate(
        pattern = r"^(0[1-9]|[12][0-9]|3[01])\-(0[1-9]|1[012])\-\d{4}$",
        message = "to_date format must be dd-mm-yyyy"
    )]
    pub to_date: Option<String>,
    #[validate(custom = segment_validation)]
    pub segment: SegmentType,
    #[validate(
        pattern = r"^(0|[1-9][0-9]*)$",
        message = "financial_year format must be concatenation of last 2 digits of from year and to year"
    )]
    pub financial_year: String,
}
