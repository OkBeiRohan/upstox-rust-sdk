use {
    crate::{
        client::ApiClient,
        constants::{
            INSTRUMENTS_ARCHIVE_FILENAME, INSTRUMENTS_COMPLETE_URL, INSTRUMENTS_JSON_FILENAME,
        },
        models::{ExchangeSegment, instruments::instruments_response::InstrumentsResponse},
    },
    flate2::read::GzDecoder,
    reqwest::{Client, Response},
    std::{
        collections::HashMap,
        fs::File,
        io::{Read, copy},
    },
    tokio::fs,
    tracing::info,
};

/// Parse the decompressed instruments JSON, surfacing the actual
/// serde error (variant + line/column) instead of the historical
/// opaque "Failed to parse Instruments JSON" string.
///
/// On `Vec<InstrumentsResponse>` failure we additionally walk the
/// raw JSON array element-by-element so the operator can see the
/// first row that does not fit any `#[serde(untagged)]` variant —
/// that row is almost always a new exchange / segment / asset_type
/// the SDK has not been taught yet.
fn parse_instruments_json(json: &str) -> Result<Vec<InstrumentsResponse>, String> {
    match serde_json::from_str::<Vec<InstrumentsResponse>>(json) {
        Ok(v) => Ok(v),
        Err(e) => {
            let mut msg = format!(
                "Failed to parse Instruments JSON into Vec<InstrumentsResponse> \
                 at line {} column {}: {e}",
                e.line(),
                e.column()
            );
            // Re-walk the array in raw form to pinpoint the first
            // offending element + dump its raw JSON.
            if let Ok(raw) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
                for (idx, value) in raw.iter().enumerate() {
                    if let Err(elem_err) =
                        serde_json::from_value::<InstrumentsResponse>(value.clone())
                    {
                        let raw_pretty = serde_json::to_string_pretty(value)
                            .unwrap_or_else(|_| value.to_string());
                        msg.push_str(&format!(
                            "\n  → first un-mappable element is index {idx}: {elem_err}\
                             \n  raw element JSON:\n{raw_pretty}"
                        ));
                        break;
                    }
                }
            }
            Err(msg)
        }
    }
}

impl ApiClient {
    pub async fn get_instruments(&self) -> Result<Vec<InstrumentsResponse>, String> {
        let client: &Client = &self.client;
        let archive_path: &str = INSTRUMENTS_ARCHIVE_FILENAME;
        let json_path: &str = INSTRUMENTS_JSON_FILENAME;
        let url: &str = INSTRUMENTS_COMPLETE_URL;

        if File::open(json_path).is_ok() {
            let mut json_file: File = File::open(json_path)
                .map_err(|e| format!("Failed to open JSON file `{json_path}`: {e}"))?;
            let mut json_content: String = String::new();
            json_file
                .read_to_string(&mut json_content)
                .map_err(|e| format!("Failed to read JSON file `{json_path}`: {e}"))?;
            return parse_instruments_json(&json_content);
        }

        let archive_file: File = match File::open(archive_path) {
            Ok(file) => Ok(file),
            Err(_) => {
                let user_agent: &str =
                    "Mozilla/5.0 (X11; Linux x86_64; rv:136.0) Gecko/20100101 Firefox/136.0";
                let accept_header: &str =
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
                let accept_encoding_header: &str = "gzip, deflate, br, zstd";

                let response: Response = client
                    .get(url)
                    .header("User-Agent", user_agent)
                    .header("Accept", accept_header)
                    .header("Accept-Encoding", accept_encoding_header)
                    .send()
                    .await
                    .map_err(|e| format!("Failed to fetch instruments from `{url}`: {e}"))?;
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| format!("Failed to read response bytes: {e}"))?;
                fs::write(archive_path, &bytes)
                    .await
                    .map_err(|e| format!("Failed to write archive `{archive_path}`: {e}"))?;
                File::open(archive_path)
                    .map_err(|e| format!("Failed to open archive `{archive_path}`: {e}"))
            }
        }?;
        info!("Instruments archive downloaded");

        let mut archive: GzDecoder<File> = GzDecoder::new(archive_file);
        let mut output_file: File = File::create(json_path)
            .map_err(|e| format!("Failed to create JSON file `{json_path}`: {e}"))?;
        copy(&mut archive, &mut output_file)
            .map_err(|e| format!("Failed to extract archive `{archive_path}`: {e}"))?;

        fs::remove_file(archive_path)
            .await
            .map_err(|e| format!("Failed to delete archive `{archive_path}`: {e}"))?;

        let mut json_file: File = File::open(json_path)
            .map_err(|e| format!("Failed to open JSON file `{json_path}`: {e}"))?;
        let mut json_content: String = String::new();
        json_file
            .read_to_string(&mut json_content)
            .map_err(|e| format!("Failed to read JSON file `{json_path}`: {e}"))?;
        // Keep the decompressed JSON on disk on parse failure so we can
        // inspect the offending row; only delete after a successful parse.
        let parsed = parse_instruments_json(&json_content)?;
        fs::remove_file(json_path)
            .await
            .map_err(|e| format!("Failed to delete JSON file `{json_path}`: {e}"))?;
        Ok(parsed)
    }

    pub fn parse_instruments(
        instruments: Vec<InstrumentsResponse>,
    ) -> HashMap<ExchangeSegment, HashMap<String, Vec<InstrumentsResponse>>> {
        let mut map: HashMap<ExchangeSegment, HashMap<String, Vec<InstrumentsResponse>>> =
            HashMap::new();

        for instrument in instruments {
            let (segment, instrument_type) = match &instrument {
                InstrumentsResponse::EquityResponse {
                    segment,
                    instrument_type,
                    ..
                } => (segment.clone(), instrument_type.clone()),
                InstrumentsResponse::DerivativeResponse {
                    segment,
                    instrument_type,
                    ..
                } => (segment.clone(), instrument_type.clone()),
                InstrumentsResponse::IndexResponse {
                    segment,
                    instrument_type,
                    ..
                } => (segment.clone(), instrument_type.clone()),
                InstrumentsResponse::CommodityResponse {
                    segment,
                    instrument_type,
                    ..
                } => (segment.clone(), instrument_type.clone()),
            };

            let segment_map: &mut HashMap<String, Vec<InstrumentsResponse>> =
                map.entry(segment).or_insert_with(HashMap::new);
            segment_map
                .entry(instrument_type)
                .or_insert_with(Vec::new)
                .push(instrument);
        }

        map
    }
}
