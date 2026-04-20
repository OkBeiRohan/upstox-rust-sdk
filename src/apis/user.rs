use crate::{
    client::ApiClient,
    constants::{
        APIVersion, BaseUrlType, USER_GET_FUND_AND_MARGIN_ENDPOINT,
        USER_GET_FUND_AND_MARGIN_V3_ENDPOINT, USER_GET_PROFILE_ENDPOINT,
    },
    models::{
        error_response::ErrorResponse,
        success_response::SuccessResponse,
        user::{
            fund_and_margin_request::{FundAndMarginRequest, SegmentType},
            fund_and_margin_response::FundAndMarginResponse,
            fund_and_margin_v3_response::FundAndMarginV3Response,
            profile_response::ProfileResponse,
        },
    },
    rate_limiter::RateLimitExceeded,
    utils::ToKeyValueTuples,
};

impl ApiClient {
    pub async fn get_profile(
        &self,
    ) -> Result<Result<SuccessResponse<ProfileResponse>, ErrorResponse>, RateLimitExceeded> {
        let res: reqwest::Response = self
            .get(
                USER_GET_PROFILE_ENDPOINT,
                true,
                None,
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;
        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<ProfileResponse>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }

    #[deprecated(
        note = "Use `get_fund_and_margin_v3` — the V2 response shape changed on \
                2025-07-19 (combined equity+commodity surfaced in the `equity` \
                object, `commodity` kept but zeroed) and the V3 endpoint exposes \
                the cash/pledge split the 2026-04-10 rollout added."
    )]
    pub async fn get_fund_and_margin(
        &self,
        segment: Option<SegmentType>,
    ) -> Result<Result<SuccessResponse<FundAndMarginResponse>, ErrorResponse>, RateLimitExceeded>
    {
        let fund_and_margin_params: FundAndMarginRequest = FundAndMarginRequest { segment };

        let res: reqwest::Response = self
            .get(
                USER_GET_FUND_AND_MARGIN_ENDPOINT,
                true,
                Some(&fund_and_margin_params.to_key_value_tuples_vec()),
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<FundAndMarginResponse>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }

    /// V3 Get Fund and Margin (2026-04-10). No `segment` parameter; the
    /// response exposes a nested `available_to_trade.{cash,pledge}` split
    /// plus `unavailable_to_trade.*` margin breakdown.
    ///
    /// Reference: <https://upstox.com/developer/api-documentation/announcements/get-funds-and-margin-v3/>
    pub async fn get_fund_and_margin_v3(
        &self,
    ) -> Result<Result<SuccessResponse<FundAndMarginV3Response>, ErrorResponse>, RateLimitExceeded>
    {
        let res: reqwest::Response = self
            .get(
                USER_GET_FUND_AND_MARGIN_V3_ENDPOINT,
                true,
                None,
                BaseUrlType::REGULAR,
                APIVersion::V3,
            )
            .await?;

        Ok(match res.status().as_u16() {
            200 => Ok(res
                .json::<SuccessResponse<FundAndMarginV3Response>>()
                .await
                .unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        })
    }
}
