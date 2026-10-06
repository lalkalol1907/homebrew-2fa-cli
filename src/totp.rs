use std::fs;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, bail};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Builder, Secret, Totp};
use url::Url;

#[derive(Debug, Clone)]
pub struct Code {
    pub code: String,
    pub ttl: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credential {
    pub secret: String,
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    #[serde(default = "default_digits")]
    pub digits: u8,
    #[serde(default = "default_period")]
    pub period: u64,
}

fn default_algorithm() -> String {
    "SHA1".into()
}

fn default_digits() -> u8 {
    6
}

fn default_period() -> u64 {
    30
}

impl Credential {
    pub fn summary(&self) -> String {
        format!(
            "{}, {} digits, {}s",
            self.algorithm, self.digits, self.period
        )
    }

    pub fn to_storage(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn from_storage(value: &str) -> anyhow::Result<Self> {
        if let Ok(credential) = serde_json::from_str::<Self>(value)
            && !credential.secret.is_empty()
        {
            return Ok(credential);
        }
        Ok(Self {
            secret: normalize_base32(value)?,
            algorithm: default_algorithm(),
            digits: default_digits(),
            period: default_period(),
        })
    }
}

pub fn parse_secret_input(input: &str) -> anyhow::Result<Credential> {
    let input = input.trim().trim_matches(|c| c == '"' || c == '\'');
    if input.is_empty() {
        bail!("secret is empty");
    }

    if is_data_image_uri(input) {
        let payload = decode_qr_from_image(&decode_data_uri(input)?)?;
        return parse_otpauth_or_base32(&payload);
    }

    if Path::new(input).is_file() {
        return parse_secret_file(Path::new(input));
    }

    parse_otpauth_or_base32(input)
}

pub fn current_code(credential: &Credential) -> anyhow::Result<Code> {
    let totp = totp_from_credential(credential)?;
    Ok(Code {
        code: totp.generate_current().to_string(),
        ttl: totp.ttl(),
    })
}

#[cfg(test)]
pub fn generate_at(secret_bytes: &[u8], unix_time: u64) -> String {
    totp_from_bytes(secret_bytes)
        .generate(unix_time)
        .to_string()
}

fn parse_secret_file(path: &Path) -> anyhow::Result<Credential> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    if let Ok(text) = std::str::from_utf8(&bytes) {
        let text = text.trim();
        if is_data_image_uri(text) || is_otpauth(text) {
            return parse_secret_input(text);
        }
    }
    let payload = decode_qr_from_image(&bytes)
        .with_context(|| format!("no QR code in {}", path.display()))?;
    parse_otpauth_or_base32(&payload)
}

fn parse_otpauth_or_base32(input: &str) -> anyhow::Result<Credential> {
    let input = input.trim();
    if !is_otpauth(input) {
        return Ok(Credential {
            secret: normalize_base32(input)?,
            algorithm: default_algorithm(),
            digits: default_digits(),
            period: default_period(),
        });
    }

    let url = Url::parse(input).context("invalid otpauth URL")?;
    if url.scheme() != "otpauth" {
        bail!("expected otpauth:// URL");
    }
    if url.host_str() != Some("totp") {
        bail!("only otpauth://totp/... is supported");
    }

    let mut secret = None;
    let mut algorithm = default_algorithm();
    let mut digits = default_digits();
    let mut period = default_period();

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "secret" => secret = Some(normalize_base32(&value)?),
            "algorithm" => algorithm = parse_algorithm_name(&value)?,
            "digits" => {
                digits = value.parse().context("invalid digits in otpauth URL")?;
                if !(6..=8).contains(&digits) {
                    bail!("digits must be 6, 7, or 8");
                }
            }
            "period" => {
                period = value.parse().context("invalid period in otpauth URL")?;
                if period == 0 {
                    bail!("period must be greater than 0");
                }
            }
            _ => {}
        }
    }

    Ok(Credential {
        secret: secret.context("otpauth URL missing secret parameter")?,
        algorithm,
        digits,
        period,
    })
}

fn parse_algorithm_name(value: &str) -> anyhow::Result<String> {
    let compact: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let algorithm = Algorithm::from_str(&compact).map_err(|_| {
        anyhow::anyhow!("unsupported algorithm '{value}', expected SHA1, SHA256, or SHA512")
    })?;
    Ok(algorithm.as_str().to_string())
}

fn is_otpauth(input: &str) -> bool {
    input.len() >= 10 && input[..10].eq_ignore_ascii_case("otpauth://")
}

fn is_data_image_uri(input: &str) -> bool {
    input.len() >= 11 && input[..11].eq_ignore_ascii_case("data:image/")
}

fn decode_data_uri(input: &str) -> anyhow::Result<Vec<u8>> {
    let (meta, payload) = input
        .split_once(',')
        .context("data URI is missing base64 payload (expected data:image/png;base64,...)")?;
    if !meta.to_ascii_lowercase().contains("base64") {
        bail!("data URI must be base64-encoded");
    }
    let compact: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    decode_base64(&compact).context("invalid base64 in data URI")
}

fn decode_base64(input: &str) -> anyhow::Result<Vec<u8>> {
    STANDARD
        .decode(input)
        .or_else(|_| URL_SAFE.decode(input))
        .context("invalid base64")
}

fn decode_qr_from_image(bytes: &[u8]) -> anyhow::Result<String> {
    let img = image::load_from_memory(bytes)
        .context("failed to decode image")?
        .to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(img);
    let grids = prepared.detect_grids();
    if grids.is_empty() {
        bail!("no QR code found in image");
    }
    let (_, content) = grids[0]
        .decode()
        .map_err(|err| anyhow::anyhow!("failed to decode QR code: {err}"))?;
    if content.trim().is_empty() {
        bail!("QR code is empty");
    }
    Ok(content)
}

fn totp_from_credential(credential: &Credential) -> anyhow::Result<Totp> {
    let algorithm = Algorithm::from_str(&credential.algorithm).map_err(|_| {
        anyhow::anyhow!(
            "unsupported algorithm '{}', expected SHA1, SHA256, or SHA512",
            credential.algorithm
        )
    })?;
    let secret = Secret::try_from_base32(&credential.secret).context("invalid base32 secret")?;
    Ok(Builder::new()
        .with_algorithm(algorithm)
        .with_digits(credential.digits)
        .with_step_duration(credential.period)
        .with_secret(secret)
        .build_noncompliant())
}

#[cfg(test)]
fn totp_from_bytes(secret_bytes: &[u8]) -> Totp {
    Builder::new()
        .with_secret(secret_bytes.to_vec())
        .build_noncompliant()
}

fn normalize_base32(input: &str) -> anyhow::Result<String> {
    let cleaned: String = input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| matches!(c, 'A'..='Z' | '2'..='7'))
        .collect();
    if cleaned.len() < 8 {
        bail!("secret is too short");
    }
    Secret::try_from_base32(&cleaned).context("invalid base32 secret")?;
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use base64::engine::general_purpose::STANDARD;
    use image::{ImageFormat, Luma};
    use qrcode::QrCode;

    // RFC 6238 Appendix B, SHA-1. 8-digit vectors truncated to 6 digits.
    const RFC6238_SECRET: &[u8] = b"12345678901234567890";
    const OTP_SECRET: &str = "JBSWY3DPEHPK3PXP";
    const OTP_URI: &str = "otpauth://totp/GitHub:user?secret=JBSWY3DPEHPK3PXP&issuer=GitHub";

    #[test]
    fn rfc6238_sha1_6_digits() {
        assert_eq!(generate_at(RFC6238_SECRET, 59), "287082");
        assert_eq!(generate_at(RFC6238_SECRET, 1_111_111_109), "081804");
        assert_eq!(generate_at(RFC6238_SECRET, 1_111_111_111), "050471");
        assert_eq!(generate_at(RFC6238_SECRET, 1_234_567_890), "005924");
        assert_eq!(generate_at(RFC6238_SECRET, 2_000_000_000), "279037");
    }

    #[test]
    fn parses_base32_and_otpauth() {
        let parsed = parse_secret_input(OTP_SECRET).unwrap();
        assert_eq!(parsed.secret, OTP_SECRET);
        assert_eq!(parsed.algorithm, "SHA1");
        assert_eq!(parsed.digits, 6);
        assert_eq!(parsed.period, 30);

        assert_eq!(
            parse_secret_input(" jbswy3dpehpk3pxp ").unwrap().secret,
            OTP_SECRET
        );
        assert_eq!(
            parse_secret_input("JBSWY3DPEHPK3PXP======").unwrap().secret,
            OTP_SECRET
        );
        assert_eq!(parse_secret_input(OTP_URI).unwrap().secret, OTP_SECRET);
    }

    #[test]
    fn honors_otpauth_parameters() {
        let uri = "otpauth://totp/GitHub:user?secret=JBSWY3DPEHPK3PXP&algorithm=SHA-256&digits=8&period=60";
        let parsed = parse_secret_input(uri).unwrap();
        assert_eq!(parsed.secret, OTP_SECRET);
        assert_eq!(parsed.algorithm, "SHA256");
        assert_eq!(parsed.digits, 8);
        assert_eq!(parsed.period, 60);

        let sha1 = totp_from_credential(&Credential {
            secret: OTP_SECRET.into(),
            algorithm: "SHA1".into(),
            digits: 6,
            period: 30,
        })
        .unwrap()
        .generate(59)
        .to_string();
        let sha256 = totp_from_credential(&parsed)
            .unwrap()
            .generate(59)
            .to_string();
        assert_ne!(sha1, sha256);
        assert_eq!(sha256.len(), 8);
    }

    #[test]
    fn parses_qr_data_uri() {
        let uri = qr_data_uri(OTP_URI);
        assert_eq!(parse_secret_input(&uri).unwrap().secret, OTP_SECRET);
    }

    #[test]
    fn rejects_hotp_and_missing_secret() {
        assert!(parse_secret_input("otpauth://hotp/GitHub:user?secret=JBSWY3DPEHPK3PXP").is_err());
        assert!(parse_secret_input("otpauth://totp/GitHub:user?issuer=GitHub").is_err());
        assert!(parse_secret_input("!!!!").is_err());
        assert!(parse_secret_input("data:image/png;base64").is_err());
    }

    fn qr_data_uri(payload: &str) -> String {
        let code = QrCode::new(payload.as_bytes()).unwrap();
        let image = code.render::<Luma<u8>>().min_dimensions(200, 200).build();
        let mut png = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();
        format!("data:image/png;base64,{}", STANDARD.encode(png))
    }
}
