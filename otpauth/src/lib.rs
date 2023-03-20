#![allow(clippy::needless_return)]

mod base32;
mod sha1;
pub mod totp;

use url::{form_urlencoded, Host, Url};

pub use totp::TotpToken;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ParseError {
    InvalidUrl,
    InvalidScheme,
    InvalidDomain,
    IncompleteQuery,
    NoIssuer,
}

#[derive(Debug, Clone)]
pub struct ParsedUrl {
    pub label: String,
    pub issuer: String,
    pub secret: Vec<u8>,
}

impl ParsedUrl {
    pub fn parse(path: &str) -> Result<ParsedUrl, ParseError> {
        let res = Url::parse(path).map_err(|_err| {
            return ParseError::InvalidUrl;
        })?;

        if res.scheme() != "otpauth" {
            return Err(ParseError::InvalidScheme);
        }

        if res.host() != Some(Host::Domain("totp")) {
            return Err(ParseError::InvalidDomain);
        }

        let label = urlencoding::decode(res.path().trim_start_matches('/'))
            .map_err(|_err| {
                return ParseError::InvalidUrl;
            })?
            .into_owned();

        let mut issuer = None;
        let mut secret = None;

        let query = res.query().ok_or(ParseError::IncompleteQuery)?;
        for (key, val) in form_urlencoded::parse(query.as_ref()) {
            if key == "secret" {
                secret = Some(base32::b32decode(val.as_ref().as_bytes()).map_err(|_err| {
                    return ParseError::IncompleteQuery;
                })?);
            } else if key == "issuer" {
                issuer = Some(val.into_owned());
            }
        }

        // The issuer sometimes need to be inferred from the label, namely, it follows
        // the pattern "{issuer}:{name}"
        if issuer.is_none() {
            issuer = Some(
                label
                    .split_once(':')
                    .ok_or(ParseError::NoIssuer)?
                    .0
                    .to_string(),
            );
        }

        return Ok(ParsedUrl {
            label,
            issuer: issuer.ok_or(ParseError::IncompleteQuery)?,
            secret: secret.ok_or(ParseError::IncompleteQuery)?,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issuer_is_a_query_value() {
        let res = ParsedUrl::parse("otpauth://totp/Company%3Aexample%40company.com?secret=gkjeixzp5xmm37meoimq====&issuer=BigTech").unwrap();
        assert_eq!(res.label.as_str(), "Company:example@company.com");
        assert_eq!(res.issuer.as_str(), "BigTech");
        assert_eq!(
            res.secret.as_slice(),
            b"\x32\x92\x44\x5F\x2F\xED\xD8\xCD\xFD\x84\x72\x19"
        );
    }

    #[test]
    fn issuer_is_inferred_from_label() {
        let res = ParsedUrl::parse(
            "otpauth://totp/Company%3Aexample%40company.com?secret=gkjeixzp5xmm37meoimq====",
        )
        .unwrap();
        assert_eq!(res.label.as_str(), "Company:example@company.com");
        assert_eq!(res.issuer.as_str(), "Company");
        assert_eq!(
            res.secret.as_slice(),
            b"\x32\x92\x44\x5F\x2F\xED\xD8\xCD\xFD\x84\x72\x19"
        );
    }

    #[test]
    fn no_issuer_is_detected() {
        ParsedUrl::parse("otpauth://totp/example%40company.com?secret=gkjeixzp5xmm37meoimq====")
            .unwrap_err();
    }

    #[test]
    fn invalid_scheme() {
        assert_eq!(
            ParsedUrl::parse(
                "http://totp/Company%3Aexample%40company.com?secret=gkjeixzp5xmm37meoimq===="
            )
            .unwrap_err(),
            ParseError::InvalidScheme,
        );
    }

    #[test]
    fn invalid_domain() {
        assert_eq!(
            ParsedUrl::parse(
                "otpauth://example/Company%3Aexample%40company.com?secret=gkjeixzp5xmm37meoimq===="
            )
            .unwrap_err(),
            ParseError::InvalidDomain,
        );
    }
}
