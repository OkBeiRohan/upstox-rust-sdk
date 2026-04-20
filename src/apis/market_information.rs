use {
    crate::{
        client::ApiClient,
        constants::{
            APIVersion, BaseUrlType, MARKET_INFO_EXCHANGE_STATUS_ENDPOINT,
            MARKET_INFO_HOLIDAYS_ENDPOINT, MARKET_INFO_TIMINGS_ENDPOINT,
        },
        models::{
            error_response::ErrorResponse,
            market_information::{
                exchange_status_request::ExchangeStatusRequest,
                exchange_status_response::ExchangeStatusResponse,
                market_holidays_request::MarketHolidaysRequest,
                market_holidays_response::MarketHolidayResponse,
                market_timings_request::MarketTimingsRequest,
                market_timings_response::MarketTimingResponse,
            },
            success_response::SuccessResponse,
        },
        rate_limiter::RateLimitExceeded,
    },
    serde_valid::Validate,
};

impl ApiClient {
    pub async fn get_market_holidays(
        &self,
        market_holidays_path_params: MarketHolidaysRequest,
    ) -> Result<Result<SuccessResponse<Vec<MarketHolidayResponse>>, ErrorResponse>, RateLimitExceeded>
    {
        market_holidays_path_params.validate().unwrap();

        let res: reqwest::Response = self
            .get(
                format!(
                    "{}/{}",
                    MARKET_INFO_HOLIDAYS_ENDPOINT,
                    match market_holidays_path_params.date {
                        Some(date) => date,
                        None => "".to_string(),
                    }
                )
                .as_str(),
                false,
                None,
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<Vec<MarketHolidayResponse>>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }

    pub async fn get_market_timings(
        &self,
        market_timings_path_params: MarketTimingsRequest,
    ) -> Result<Result<SuccessResponse<Vec<MarketTimingResponse>>, ErrorResponse>, RateLimitExceeded>
    {
        market_timings_path_params.validate().unwrap();

        let res: reqwest::Response = self
            .get(
                format!(
                    "{}/{}",
                    MARKET_INFO_TIMINGS_ENDPOINT, market_timings_path_params.date
                )
                .as_str(),
                false,
                None,
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<Vec<MarketTimingResponse>>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }

    pub async fn get_exchange_status(
        &self,
        exchange_status_path_params: ExchangeStatusRequest,
    ) -> Result<Result<SuccessResponse<ExchangeStatusResponse>, ErrorResponse>, RateLimitExceeded>
    {
        let res: reqwest::Response = self
            .get(
                format!(
                    "{}/{}",
                    MARKET_INFO_EXCHANGE_STATUS_ENDPOINT, exchange_status_path_params.exchange
                )
                .as_str(),
                true,
                None,
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<ExchangeStatusResponse>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }
}
