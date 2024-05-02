#![allow(clippy::needless_return)]

use crate::base32;
use url::{form_urlencoded, Host, Url};

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
    pub account_name: String,
    pub issuer: String,
    pub secret: Vec<u8>,
    pub period: u64,
    pub digits: usize,
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

        // The issuer can be specified in the label with the pattern "{issuer}:{name}".
        // If this is the case and the label query parameter is present, we ensure they
        // are equal.
        let (mut issuer, account_name) = match label.split_once(':') {
            None => (None, label),
            Some((issuer, account_name)) => (Some(issuer.to_string()), account_name.to_string())
        };

        let mut secret = None;
        let mut digits = 6;
        let mut period = 30;

        let query = res.query().ok_or(ParseError::IncompleteQuery)?;
        for (key, val) in form_urlencoded::parse(query.as_ref()) {
            if key == "secret" {
                secret = Some(base32::b32decode(val.as_ref().as_bytes()).map_err(|_err| {
                    return ParseError::IncompleteQuery;
                })?);
            } else if key == "issuer" {
                issuer = Some(val.into_owned());
            } else if key == "digits" {
                digits = usize::from_str_radix(&val, 10).map_err(|err| {
                    eprintln!("Failed to parse '{}' as usize in base 10, err: {}", val, err);
                    return ParseError::InvalidUrl;
                })?;
            } else if key == "period" {
                period = u64::from_str_radix(&val, 10).map_err(|err| {
                    eprintln!("Failed to parse '{}' as u64 in base 10, err: {}", val, err);
                    return ParseError::InvalidUrl;
                })?;
            }
        }

        if issuer.is_none() {
            return Err(ParseError::NoIssuer);
        }

        return Ok(ParsedUrl {
            account_name,
            issuer: issuer.ok_or(ParseError::IncompleteQuery)?,
            secret: secret.ok_or(ParseError::IncompleteQuery)?,
            period,
            digits,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issuer_is_a_query_value() {
        let res = ParsedUrl::parse("otpauth://totp/BigTech%3Aexample%40company.com?secret=gkjeixzp5xmm37meoimq====&issuer=BigTech&digits=10&period=35").unwrap();
        assert_eq!(res.account_name.as_str(), "example@company.com");
        assert_eq!(res.issuer.as_str(), "BigTech");
        assert_eq!(
            res.secret.as_slice(),
            b"\x32\x92\x44\x5F\x2F\xED\xD8\xCD\xFD\x84\x72\x19"
        );
        assert_eq!(res.digits, 10);
        assert_eq!(res.period, 35);
    }

    #[test]
    fn issuer_is_inferred_from_label() {
        let res = ParsedUrl::parse(
            "otpauth://totp/Company%3Aexample%40company.com?secret=gkjeixzp5xmm37meoimq====",
        )
        .unwrap();
        assert_eq!(res.account_name.as_str(), "example@company.com");
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
