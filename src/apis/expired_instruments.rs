use {
    crate::{
        client::ApiClient,
        constants::{
            APIVersion, BaseUrlType, EXPIRED_INSTRUMENTS_EXPIRED_FUTURE_CONTRACTS_ENDPOINT,
            EXPIRED_INSTRUMENTS_EXPIRED_HISTORICAL_CANDLE_DATA_ENDPOINT,
            EXPIRED_INSTRUMENTS_EXPIRED_OPTION_CONTRACTS_ENDPOINT,
            EXPIRED_INSTRUMENTS_EXPIRIES_ENDPOINT,
        },
        models::{
            error_response::ErrorResponse,
            expired_instruments::{
                expired_derivative_contracts_request::ExpiredDerivativeContractsRequest,
                expired_future_contracts_response::ExpiredFutureContractsResponse,
                expired_historical_candle_data_request::ExpiredHistoricalCandleDataRequest,
                expiries_request::ExpiriesRequest,
            },
            historical_data::candle_data_response::CandleDataResponse,
            option_chain::option_contracts_response::OptionContractsResponse,
            success_response::SuccessResponse,
        },
        rate_limiter::RateLimitExceeded,
        utils::ToKeyValueTuples,
    },
    serde_valid::Validate,
};

impl ApiClient {
    pub async fn get_expiries(
        &self,
        expiries_params: ExpiriesRequest,
    ) -> Result<Result<SuccessResponse<Vec<String>>, ErrorResponse>, RateLimitExceeded> {
        // Expired-instruments APIs are Plus-only (Upstox announcement
        // 2025-05-13). Refuse up-front on the standard tier so
        // operators see a typed error rather than a silent
        // 4xx / empty-response pair from the broker.
        self.ensure_plus_user("expired-instruments/expiries")?;
        expiries_params.validate().unwrap();

        let res: reqwest::Response = self
            .get(
                EXPIRED_INSTRUMENTS_EXPIRIES_ENDPOINT,
                true,
                Some(&expiries_params.to_key_value_tuples_vec()),
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res.json::<SuccessResponse<Vec<String>>>().await.unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }

    pub async fn get_expired_option_contracts(
        &self,
        expired_option_contracts_params: ExpiredDerivativeContractsRequest,
    ) -> Result<
        Result<SuccessResponse<Vec<OptionContractsResponse>>, ErrorResponse>,
        RateLimitExceeded,
    > {
        self.ensure_plus_user("expired-instruments/option-contract")?;
        expired_option_contracts_params.validate().unwrap();

        let res: reqwest::Response = self
            .get(
                EXPIRED_INSTRUMENTS_EXPIRED_OPTION_CONTRACTS_ENDPOINT,
                true,
                Some(&expired_option_contracts_params.to_key_value_tuples_vec()),
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<Vec<OptionContractsResponse>>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }

    pub async fn get_expired_future_contracts(
        &self,
        expired_future_contracts_params: ExpiredDerivativeContractsRequest,
    ) -> Result<
        Result<SuccessResponse<Vec<ExpiredFutureContractsResponse>>, ErrorResponse>,
        RateLimitExceeded,
    > {
        self.ensure_plus_user("expired-instruments/future-contract")?;
        expired_future_contracts_params.validate().unwrap();

        let res: reqwest::Response = self
            .get(
                EXPIRED_INSTRUMENTS_EXPIRED_FUTURE_CONTRACTS_ENDPOINT,
                true,
                Some(&expired_future_contracts_params.to_key_value_tuples_vec()),
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<Vec<ExpiredFutureContractsResponse>>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }

    pub async fn get_expired_historical_candle_data(
        &self,
        expired_historical_candles_path_params: ExpiredHistoricalCandleDataRequest,
    ) -> Result<Result<SuccessResponse<CandleDataResponse>, ErrorResponse>, RateLimitExceeded> {
        self.ensure_plus_user("expired-instruments/historical-candle")?;
        expired_historical_candles_path_params.validate().unwrap();

        let res: reqwest::Response = self
            .get(
                format!(
                    "{}/{}/{}/{}/{}",
                    EXPIRED_INSTRUMENTS_EXPIRED_HISTORICAL_CANDLE_DATA_ENDPOINT,
                    expired_historical_candles_path_params.expired_instrument_key,
                    expired_historical_candles_path_params.interval,
                    expired_historical_candles_path_params.to_date,
                    expired_historical_candles_path_params.from_date
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
                .json::<SuccessResponse<CandleDataResponse>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }
}
