#[derive(Clone)]
pub struct EslClientConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub event_format: super::esl::EslEventFormat,
}
