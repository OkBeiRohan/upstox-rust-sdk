use {
    crate::{
        client::{ApiClient, AutomateLoginConfig, LoginConfig, MailProvider},
        constants::{
            APIVersion, BaseUrlType, EMAIL_ID_ENV, GOOGLE_AUTHORIZATION_CODE_ENV,
            GOOGLE_CLIENT_ID_ENV, GOOGLE_CLIENT_SECRET_ENV, GOOGLE_IMAP_URL,
            GOOGLE_OAUTH2_ACCESS_TOKEN_URL, GOOGLE_OAUTH2_AUTH_URL, GOOGLE_REFRESH_TOKEN_FILENAME,
            LOGIN_AUTHORIZE_ENDPOINT, LOGIN_GET_TOKEN_ENDPOINT, LOGIN_PIN_ENV, LOGOUT_ENDPOINT,
            MOBILE_NUMBER_ENV, REDIRECT_PORT_ENV, UPLINK_API_KEY_ENV, UPLINK_API_SECRET_ENV,
            UPSTOX_ACCESS_TOKEN_FILENAME, WEBDRIVER_SOCKET_ENV,
        },
        models::{
            error_response::ErrorResponse,
            login::{
                dialog_request::{DialogRequest, ResponseType},
                google_oauth2_request::{
                    self, AccessType, GoogleOAuth2AuthRequest, GoogleOAuth2CodeTokenRequest,
                    GoogleOAuth2RefreshTokenRequest, Prompt,
                },
                google_oauth2_response::{
                    GoogleOAuth2TokenErrorResponse, GoogleOAuth2TokenResponse,
                },
                token_request::{self, TokenRequest},
                token_response::TokenResponse,
            },
            success_response::SuccessResponse,
        },
        utils::{ToKeyValueTuples, create_url, read_value_from_file, write_value_to_file},
    },
    async_imap::{
        self, Authenticator, Client as ImapClient, Session,
        types::{Fetch, Mailbox},
    },
    async_native_tls::{TlsConnector, TlsStream},
    chrono::{DateTime, Utc},
    fantoccini::{Client as FantocciniClient, ClientBuilder, Locator, elements::Element},
    futures::TryStreamExt,
    mailparse::{ParsedMail, parse_mail},
    regex::Regex,
    reqwest::Url,
    scraper::{ElementRef, Html, Selector},
    std::{self, borrow::Cow, env, fs::remove_file, net::SocketAddr, sync::Arc},
    tokio::{
        self,
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::{Mutex, MutexGuard},
        time::{Duration, sleep},
    },
    tracing::{debug, info, warn},
    url_open::UrlOpen,
    urlencoding::decode,
};

#[derive(Debug)]
struct OAuth2 {
    user: String,
    access_token: String,
}

impl Authenticator for &OAuth2 {
    type Response = String;
    fn process(&mut self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

impl ApiClient {
    pub(crate) async fn login(&mut self, login_config: &LoginConfig) -> Result<(), String> {
        if let Ok(access_token) = read_value_from_file(UPSTOX_ACCESS_TOKEN_FILENAME) {
            self.token = Some(access_token);
            if self.verify_authorization().await {
                return Ok(());
            }
        };

        if login_config.automate_login_config.is_none() {
            return Err("Must provide automate_login_config for authorization.".to_string());
        }

        let automate_login_config: &AutomateLoginConfig =
            login_config.automate_login_config.as_ref().unwrap();

        let auth_code: String = self.get_authorization_code(automate_login_config).await?;

        match self.get_token(auth_code.to_string()).await {
            Ok(token_response) => {
                self.token = Some(token_response.access_token);
                write_value_to_file(UPSTOX_ACCESS_TOKEN_FILENAME, &self.token.as_ref().unwrap())
                    .unwrap();
                Ok(())
            }
            Err(error_response) => Err(error_response.errors[0].message.clone()),
        }
    }

    pub async fn get_authorization_code(
        &self,
        automate_login_config: &AutomateLoginConfig,
    ) -> Result<String, String> {
        let redirect_port: String = env::var(REDIRECT_PORT_ENV).unwrap();

        let dialog_request_params: DialogRequest = DialogRequest {
            client_id: self.api_key.clone(),
            redirect_uri: format!("{}{}", "http://127.0.0.1:", &redirect_port),
            state: None,
            response_type: ResponseType::Code,
        };
        let full_url: Url = Url::parse_with_params(
            create_url(
                BaseUrlType::REGULAR,
                APIVersion::V2,
                LOGIN_AUTHORIZE_ENDPOINT,
            )
            .as_str(),
            dialog_request_params.to_key_value_tuples_vec(),
        )
        .unwrap();

        if automate_login_config.automate_login {
            if automate_login_config.automate_fetching_otp
                && automate_login_config.mail_provider.is_none()
            {
                return Err(
                    "Cannot automate fetching OTP as no mail provider specified".to_string()
                );
            }

            let login_pin: String = env::var(LOGIN_PIN_ENV)
                .expect("Env variable LOGIN_PIN must be set to automate login");
            let webdriver_socket: String = env::var(WEBDRIVER_SOCKET_ENV)
                .expect("Env variable WEBDRIVER_SOCKET must be set to automate login");

            let fantoccini_client: FantocciniClient = ClientBuilder::native()
                .connect(&webdriver_socket)
                .await
                .map_err(|_| "Failed to connect to WebDriver".to_string())?;
            let fantoccini_client: Arc<Mutex<Option<FantocciniClient>>> =
                Arc::new(Mutex::new(Some(fantoccini_client)));

            {
                let mut client: tokio::sync::MutexGuard<Option<FantocciniClient>> =
                    fantoccini_client.lock().await;
                let client: &mut FantocciniClient =
                    client.as_mut().expect("Client is already closed");
                client.goto(full_url.as_str()).await.unwrap();
            }

            sleep(Duration::from_millis(200)).await;
            let otp_sent_timestamp: i64 = Utc::now().timestamp();
            self.send_otp(fantoccini_client.clone()).await;

            let automate_fetching_otp: bool = automate_login_config.automate_fetching_otp;

            if automate_fetching_otp {
                match self
                    .get_otp(
                        otp_sent_timestamp,
                        automate_login_config.mail_provider.clone().unwrap(),
                    )
                    .await
                {
                    Ok(otp) => {
                        let mut client: tokio::sync::MutexGuard<Option<FantocciniClient>> =
                            fantoccini_client.lock().await;
                        let client: &mut FantocciniClient =
                            client.as_mut().expect("Client is already closed");
                        let otp_field: Element = client
                            .wait()
                            .every(Duration::from_millis(100))
                            .for_element(Locator::Id("otpNum"))
                            .await
                            .unwrap();
                        otp_field.send_keys(&otp).await.unwrap();

                        let continue_button: Element =
                            client.find(Locator::Id("continueBtn")).await.unwrap();
                        continue_button.click().await.unwrap();
                    }
                    Err(err) => {
                        warn!("Error while fetching OTP: {}", err);
                    }
                }
            }

            let auth_code: String = {
                let mut client: tokio::sync::MutexGuard<Option<FantocciniClient>> =
                    fantoccini_client.lock().await;
                let client: &mut FantocciniClient =
                    client.as_mut().expect("Client is already closed");

                let pin_field: Element = client
                    .wait()
                    .every(Duration::from_millis(100))
                    .for_element(Locator::Id("pinCode"))
                    .await
                    .unwrap();
                pin_field.send_keys(&login_pin).await.unwrap();

                let pin_continue_button: Element =
                    client.find(Locator::Id("pinContinueBtn")).await.unwrap();

                let code_future = self.await_and_extract_code();
                let click_result = pin_continue_button.click().await;

                match click_result {
                    Ok(_) => {
                        // Wait for the server to capture the code
                        code_future.await
                    }
                    Err(err) => {
                        // Handle the error and extract the code from the error message
                        if let fantoccini::error::CmdError::Standard(webdriver_error) = err {
                            let message = webdriver_error.message.into_owned();
                            if message.contains("Reached error page") && message.contains("code%3D")
                            {
                                // Extract the code from the error message
                                if let Some(code_start) = message.find("code%3D") {
                                    let code_start = code_start + 7; // Skip "code="
                                    if let Some(code_end) = message[code_start..].find('&') {
                                        let code = &message[code_start..code_start + code_end];
                                        return Ok(code.to_string());
                                    }
                                }
                            }
                        }
                        panic!("Failed to click continue button and extract code")
                    }
                }
            };

            self.close_fantoccini_client(fantoccini_client).await;
            Ok(auth_code)
        } else {
            full_url.open();
            Ok(self.await_and_extract_code().await)
        }
    }

    pub async fn get_token(&self, auth_code: String) -> Result<TokenResponse, ErrorResponse> {
        let client_id: String = env::var(UPLINK_API_KEY_ENV).unwrap();
        let client_secret: String = env::var(UPLINK_API_SECRET_ENV).unwrap();
        let redirect_port: String = env::var(REDIRECT_PORT_ENV).unwrap();

        let token_request_form: TokenRequest = TokenRequest {
            code: auth_code,
            client_id,
            client_secret,
            redirect_uri: format!("{}{}", "http://127.0.0.1:", &redirect_port),
            grant_type: token_request::GrantType::AuthorizationCode,
        };

        let res: reqwest::Response = self
            .post::<()>(
                LOGIN_GET_TOKEN_ENDPOINT,
                false,
                None,
                Some(&token_request_form.to_key_value_tuples_vec()),
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await
            .unwrap();

        match res.status().as_u16() {
            200 => Ok(res.json::<TokenResponse>().await.unwrap()),
            _ => Err(res.json::<ErrorResponse>().await.unwrap()),
        }
    }

    pub async fn logout(&self) -> Result<SuccessResponse<bool>, String> {
        let res: reqwest::Response = self
            .delete::<()>(
                LOGOUT_ENDPOINT,
                true,
                None,
                None,
                BaseUrlType::REGULAR,
                APIVersion::V2,
            )
            .await
            .unwrap();
        match res.status().as_u16() {
            200 => Ok(res.json::<SuccessResponse<bool>>().await.unwrap()),
            _ => Err("Unexpected error while logging out".to_string()),
        }
    }

    async fn send_otp(&self, fantoccini_client: Arc<Mutex<Option<FantocciniClient>>>) {
        let mobile_number: String = env::var(MOBILE_NUMBER_ENV).unwrap();
        {
            let client: tokio::sync::MutexGuard<Option<FantocciniClient>> =
                fantoccini_client.lock().await;
            let client: &FantocciniClient = client.as_ref().expect("Client is already closed");
            let mobile_number_field: Element = client.find(Locator::Id("mobileNum")).await.unwrap();
            mobile_number_field.send_keys(&mobile_number).await.unwrap();

            let get_otp_button: Element = client.find(Locator::Id("getOtp")).await.unwrap();
            get_otp_button.click().await.unwrap();
        }
    }

    async fn get_otp(
        &self,
        otp_sent_time: i64,
        mail_provider: MailProvider,
    ) -> Result<String, String> {
        let email: String = env::var(EMAIL_ID_ENV).unwrap();
        let access_token: String = match mail_provider {
            MailProvider::Google => match self.get_google_access_token().await {
                Ok(token) => token,
                Err(_) => self.get_google_access_token().await?,
            },
        };

        let oauth2: OAuth2 = OAuth2 {
            user: email,
            access_token,
        };
        let domain: &str = match mail_provider {
            MailProvider::Google => GOOGLE_IMAP_URL,
        };
        let tcp_stream: TcpStream = TcpStream::connect((domain, 993)).await.unwrap();
        let tls_connector: TlsConnector = TlsConnector::new();
        let tls_stream: TlsStream<TcpStream> =
            tls_connector.connect(domain, tcp_stream).await.unwrap();

        let mut client: ImapClient<TlsStream<TcpStream>> = ImapClient::new(tls_stream);
        let _greeting = client
            .read_response()
            .await
            .expect("Unexpected end of stream, expected greeting");

        let mut imap_session: Session<TlsStream<TcpStream>> =
            client.authenticate("XOAUTH2", &oauth2).await.unwrap();

        info!("OTP Sent: {}", otp_sent_time);

        let mut retries: u32 = 0;
        let max_retries: u32 = 5;
        loop {
            if retries >= max_retries {
                return Err("Failed to fetch OTP automatically after multiple attempts".to_string());
            }
            retries += 1;

            let inbox: Mailbox = imap_session.select("INBOX").await.unwrap();
            let msg_count: u32 = inbox.exists;

            let start_seq_no = if msg_count > 2 { msg_count - 2 } else { 1 };
            for seq_no in (start_seq_no..=msg_count).rev() {
                let msg_headers: Option<Fetch> = self
                    .get_message_data(
                        &mut imap_session,
                        seq_no.to_string(),
                        "BODY[HEADER.FIELDS (SUBJECT FROM DATE)]",
                    )
                    .await;
                let msg_headers: Fetch = match msg_headers {
                    Some(msg_headers) => msg_headers,
                    None => break,
                };
                let headers: &str = std::str::from_utf8(msg_headers.header().unwrap()).unwrap();

                let msg_timestamp: i64 = DateTime::parse_from_rfc2822(
                    self.get_header(headers, "Date").unwrap().as_str(),
                )
                .unwrap()
                .timestamp();
                debug!("Read mail with time: {}", msg_timestamp);
                if msg_timestamp < otp_sent_time {
                    break;
                }

                let from_match: bool =
                    self.check_header(
                        headers,
                        "From",
                        "Upstox Alert <donotreply@transactions.upstox.com>",
                    ) || self.check_header(
                        headers,
                        "From",
                        "Upstox <donotreply@transactions.upstox.com>",
                    ) || self.check_header(headers, "From", "Upstox <noreply@upstox.com>");
                let subject_match: bool = self.check_header(headers, "Subject", "OTP to login");

                if from_match && subject_match {
                    debug!("Found OTP email at time: {}", msg_timestamp);
                    let msg_text: Fetch = self
                        .get_message_data(&mut imap_session, seq_no.to_string(), "BODY[TEXT]")
                        .await
                        .unwrap();
                    imap_session.logout().await.unwrap();

                    let raw_text: &[u8] = msg_text.text().unwrap();
                    let parsed_mail: ParsedMail = parse_mail(raw_text).unwrap();
                    let html_content: String = parsed_mail.get_body().unwrap();

                    let document: Html = Html::parse_document(&html_content);
                    let span_selector: Selector = Selector::parse("span").unwrap();

                    let re: Regex = Regex::new(r"[0-9]{6}").unwrap();
                    let otp_element: ElementRef = document
                        .select(&span_selector)
                        .into_iter()
                        .find(|element| match element.text().next() {
                            Some(val) => {
                                debug!("Found OTP element: {}", val);
                                re.find(val).is_some()
                            }
                            None => false,
                        })
                        .unwrap();

                    let otp: &str = otp_element.text().next().unwrap();

                    return Ok(otp.to_string());
                }
            }
            sleep(Duration::from_millis(1000)).await;
        }
    }

    async fn get_message_data(
        &self,
        imap_session: &mut Session<TlsStream<TcpStream>>,
        seq_set: String,
        query: &str,
    ) -> Option<Fetch> {
        let msgs_stream = imap_session.fetch(seq_set, query).await.unwrap();
        let msgs: Vec<Fetch> = msgs_stream.try_collect().await.unwrap();
        msgs.into_iter().next()
    }

    fn get_header(&self, headers: &str, field: &str) -> Option<String> {
        for line in headers.lines() {
            if line.to_lowercase().starts_with(&field.to_lowercase()) {
                return Some(line[field.len() + 1..].trim().to_string());
            }
        }
        None
    }

    fn check_header(&self, headers: &str, field: &str, expected_value: &str) -> bool {
        match self.get_header(headers, field) {
            Some(value) => value == expected_value,
            None => false,
        }
    }

    async fn get_google_access_token(&self) -> Result<String, String> {
        let client: &reqwest::Client = &self.client;

        let client_id: String = env::var(GOOGLE_CLIENT_ID_ENV).unwrap();
        let client_secret: String = env::var(GOOGLE_CLIENT_SECRET_ENV).unwrap();

        let google_oauth2_token_request_body: Box<dyn ToKeyValueTuples>;
        let refresh_token_found: bool;
        match read_value_from_file(GOOGLE_REFRESH_TOKEN_FILENAME) {
            // Refresh Token already present, so use it to get new access token
            Ok(refresh_token) => {
                refresh_token_found = true;
                google_oauth2_token_request_body = Box::new(GoogleOAuth2RefreshTokenRequest {
                    client_id,
                    client_secret,
                    grant_type: google_oauth2_request::GrantType::RefreshToken,
                    refresh_token,
                });
            }
            // No refresh token found, so use freshly generated authorization code from environment to generate access_token and refresh_token
            Err(_) => {
                refresh_token_found = false;
                let code: String = match env::var(GOOGLE_AUTHORIZATION_CODE_ENV) {
                    Ok(code) => code,
                    Err(_) => self.get_google_auth_code().await,
                };

                let redirect_port: String = env::var(REDIRECT_PORT_ENV).unwrap();

                google_oauth2_token_request_body = Box::new(GoogleOAuth2CodeTokenRequest {
                    client_id,
                    client_secret,
                    code,
                    code_verifier: None,
                    grant_type: google_oauth2_request::GrantType::AuthorizationCode,
                    redirect_uri: format!("{}{}", "http://127.0.0.1:", &redirect_port),
                });
            }
        }

        let res: reqwest::Response = client
            .post(GOOGLE_OAUTH2_ACCESS_TOKEN_URL)
            .form(&google_oauth2_token_request_body.to_key_value_tuples_vec())
            .send()
            .await
            .unwrap();

        match res.status().as_u16() {
            200 => {
                let response_data = res.json::<GoogleOAuth2TokenResponse>().await.unwrap();
                if !refresh_token_found {
                    let _ = write_value_to_file(
                        GOOGLE_REFRESH_TOKEN_FILENAME,
                        response_data.refresh_token.unwrap().as_str(),
                    );
                }
                return Ok(response_data.access_token);
            }
            400 => {
                let error_data: GoogleOAuth2TokenErrorResponse =
                    res.json::<GoogleOAuth2TokenErrorResponse>().await.unwrap();
                if refresh_token_found {
                    let _ = remove_file(GOOGLE_REFRESH_TOKEN_FILENAME);
                }
                return Err(error_data.error);
            }
            _ => panic!(),
        };
    }

    async fn get_google_auth_code(&self) -> String {
        let client_id: String = env::var(GOOGLE_CLIENT_ID_ENV).unwrap();
        let redirect_port: String = env::var(REDIRECT_PORT_ENV).unwrap();

        let google_oauth2_auth_request: GoogleOAuth2AuthRequest = GoogleOAuth2AuthRequest {
            client_id,
            redirect_uri: format!("{}{}", "http://127.0.0.1:", &redirect_port),
            response_type: google_oauth2_request::ResponseType::Code,
            scope: "https://mail.google.com/".to_string(),
            code_challenge: None,
            code_challenge_method: None,
            state: None,
            login_hint: None,
            access_type: Some(AccessType::Offline),
            prompt: Some(Prompt::SelectAccount),
        };

        let oauth_url: Url = Url::parse_with_params(
            GOOGLE_OAUTH2_AUTH_URL,
            google_oauth2_auth_request.to_key_value_tuples_vec(),
        )
        .unwrap();
        oauth_url.open();

        self.await_and_extract_code().await
    }

    async fn await_and_extract_code(&self) -> String {
        let redirect_port: String = env::var(REDIRECT_PORT_ENV).unwrap();

        let addr: SocketAddr =
            SocketAddr::from(([127, 0, 0, 1], str::parse::<u16>(&redirect_port).unwrap()));
        let listener: TcpListener = TcpListener::bind(addr).await.unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer: [u8; 1024] = [0; 1024];
        socket.read(&mut buffer).await.unwrap();
        let request: Cow<str> = String::from_utf8_lossy(&buffer[..]);

        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<!DOCTYPE html><html><body>You can now close this tab!</body></html>";
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.flush().await.unwrap();
        socket.shutdown().await.unwrap();
        self.parse_code(request.to_string()).unwrap()
    }

    fn parse_code(&self, request: String) -> Option<String> {
        if let Some(start_index) = request.find("code=") {
            let start_index: usize = start_index + 5;
            if let Some(end_index) = request[start_index..].find(|c| c == '&' || c == ' ') {
                let end_index: usize = start_index + end_index;
                let code: &str = &request[start_index..end_index];
                return decode(code).ok().map(|decoded| decoded.into_owned());
            }
        }

        None
    }

    async fn close_fantoccini_client(
        &self,
        fantoccini_client: Arc<Mutex<Option<FantocciniClient>>>,
    ) {
        let mut client: MutexGuard<Option<FantocciniClient>> = fantoccini_client.lock().await;
        if let Some(client) = client.take() {
            client.close().await.unwrap();
        }
    }
}
