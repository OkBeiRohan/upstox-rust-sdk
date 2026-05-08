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

impl ApiClient {
    pub async fn get_instruments(&self) -> Result<Vec<InstrumentsResponse>, String> {
        let client: &Client = &self.client;
        let archive_path: &str = INSTRUMENTS_ARCHIVE_FILENAME;
        let json_path: &str = INSTRUMENTS_JSON_FILENAME;
        let url: &str = INSTRUMENTS_COMPLETE_URL;

        if File::open(json_path).is_ok() {
            let mut json_file: File =
                File::open(json_path).map_err(|_| "Failed to open JSON file".to_string())?;
            let mut json_content: String = String::new();
            json_file
                .read_to_string(&mut json_content)
                .map_err(|_| "Failed to read JSON file")?;
            let instruments_data: Vec<InstrumentsResponse> = serde_json::from_str(&json_content)
                .map_err(|_| "Failed to parse Instruments JSON".to_string())?;
            return Ok(instruments_data);
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
                    .map_err(|_| "Failed to fetch instruments".to_string())?;
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|_| "Failed to read response bytes".to_string())?;
                fs::write(archive_path, &bytes)
                    .await
                    .map_err(|_| "Failed to write archive".to_string())?;
                File::open(archive_path).map_err(|_| "Failed to open archive".to_string())
            }
        }?;
        info!("Instruments archive downloaded");

        let mut archive: GzDecoder<File> = GzDecoder::new(archive_file);
        let mut output_file: File =
            File::create(json_path).map_err(|_| "Failed to create JSON file".to_string())?;
        copy(&mut archive, &mut output_file)
            .map_err(|_| "Failed to extract archive".to_string())?;

        fs::remove_file(archive_path)
            .await
            .expect("Failed to delete archive");

        let mut json_file: File =
            File::open(json_path).map_err(|_| "Failed to open JSON file".to_string())?;
        let mut json_content: String = String::new();
        json_file
            .read_to_string(&mut json_content)
            .map_err(|_| "Failed to read JSON file")?;
        fs::remove_file(json_path)
            .await
            .map_err(|_| "Failed to delete JSON file")?;

        let instruments_data: Vec<InstrumentsResponse> = serde_json::from_str(&json_content)
            .map_err(|_| "Failed to parse Instruments JSON".to_string())?;
        Ok(instruments_data)
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
