//! JWKS (JSON Web Key Set) support
//!
//! This module provides types and functions for parsing and working with
//! JSON Web Key Sets (JWKS), as defined in RFC 7517.

use jsonwebtoken::{Algorithm, DecodingKey};
use serde::{Deserialize, Serialize};

/// Error type for JWK key conversion
#[derive(Debug)]
pub enum JwkError {
    /// Missing required parameter for the key type
    MissingParameter(String),
    /// Unsupported key type
    UnsupportedKeyType(String),
    /// Unsupported curve for EC keys
    UnsupportedCurve(String),
    /// Failed to create decoding key
    KeyCreationFailed(String),
}

impl std::fmt::Display for JwkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwkError::MissingParameter(param) => {
                write!(f, "Missing required JWK parameter: {}", param)
            }
            JwkError::UnsupportedKeyType(kty) => {
                write!(f, "Unsupported JWK key type: {}", kty)
            }
            JwkError::UnsupportedCurve(crv) => {
                write!(f, "Unsupported EC curve: {}", crv)
            }
            JwkError::KeyCreationFailed(reason) => {
                write!(f, "Failed to create decoding key: {}", reason)
            }
        }
    }
}

impl std::error::Error for JwkError {}

/// JSON Web Key Set (JWKS) - a set of JWK keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwks {
    /// The keys in the JWKS
    pub keys: Vec<JwkKey>,
}

impl Jwks {
    /// Find a key by its Key ID (kid)
    pub fn find_key_by_kid(&self, kid: &str) -> Option<&JwkKey> {
        self.keys.iter().find(|k| k.kid.as_deref() == Some(kid))
    }

    /// Get all keys of a specific type (RSA, EC, etc.)
    pub fn keys_by_type(&self, kty: &str) -> Vec<&JwkKey> {
        self.keys.iter().filter(|k| k.kty == kty).collect()
    }
}

/// JSON Web Key (JWK) - a single key in a JWKS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkKey {
    /// Key Type (e.g., "RSA", "EC")
    pub kty: String,

    /// Key ID - unique identifier for the key
    #[serde(default)]
    pub kid: Option<String>,

    /// Public Key Use ("sig" for signature, "enc" for encryption)
    #[serde(default, rename = "use")]
    pub key_use: Option<String>,

    /// Algorithm intended for use with the key (e.g., "RS256", "ES256")
    #[serde(default)]
    pub alg: Option<String>,

    // RSA key parameters (RFC 7518)
    /// RSA modulus (base64url-encoded)
    #[serde(default)]
    pub n: Option<String>,

    /// RSA public exponent (base64url-encoded)
    #[serde(default)]
    pub e: Option<String>,

    // EC key parameters (RFC 7518)
    /// EC curve name (e.g., "P-256", "P-384")
    #[serde(default)]
    pub crv: Option<String>,

    /// EC x coordinate (base64url-encoded)
    #[serde(default)]
    pub x: Option<String>,

    /// EC y coordinate (base64url-encoded)
    #[serde(default)]
    pub y: Option<String>,
}

impl JwkKey {
    /// Check if this is an RSA key
    pub fn is_rsa(&self) -> bool {
        self.kty == "RSA"
    }

    /// Check if this is an EC (Elliptic Curve) key
    pub fn is_ec(&self) -> bool {
        self.kty == "EC"
    }

    /// Get the algorithm, defaulting based on key type if not specified
    pub fn algorithm(&self) -> Option<&str> {
        self.alg.as_deref().or(self.default_algorithm())
    }

    /// Get the default algorithm based on key type and curve
    fn default_algorithm(&self) -> Option<&'static str> {
        match self.kty.as_str() {
            "RSA" => Some("RS256"),
            "EC" => match self.crv.as_deref() {
                Some("P-256") => Some("ES256"),
                Some("P-384") => Some("ES384"),
                Some("P-521") => Some("ES512"),
                _ => None,
            },
            _ => None,
        }
    }

    /// Convert this JWK to a DecodingKey for JWT validation
    pub fn to_decoding_key(&self) -> Result<DecodingKey, JwkError> {
        match self.kty.as_str() {
            "RSA" => {
                let n = self
                    .n
                    .as_ref()
                    .ok_or_else(|| JwkError::MissingParameter("n (modulus)".to_string()))?;
                let e = self
                    .e
                    .as_ref()
                    .ok_or_else(|| JwkError::MissingParameter("e (exponent)".to_string()))?;

                DecodingKey::from_rsa_components(n, e)
                    .map_err(|e| JwkError::KeyCreationFailed(e.to_string()))
            }
            "EC" => {
                let x = self
                    .x
                    .as_ref()
                    .ok_or_else(|| JwkError::MissingParameter("x (coordinate)".to_string()))?;
                let y = self
                    .y
                    .as_ref()
                    .ok_or_else(|| JwkError::MissingParameter("y (coordinate)".to_string()))?;

                DecodingKey::from_ec_components(x, y)
                    .map_err(|e| JwkError::KeyCreationFailed(e.to_string()))
            }
            other => Err(JwkError::UnsupportedKeyType(other.to_string())),
        }
    }

    /// Get the jsonwebtoken Algorithm for this key
    pub fn jwt_algorithm(&self) -> Result<Algorithm, JwkError> {
        let alg_str = self
            .algorithm()
            .ok_or_else(|| JwkError::UnsupportedKeyType(self.kty.clone()))?;

        match alg_str {
            "RS256" => Ok(Algorithm::RS256),
            "RS384" => Ok(Algorithm::RS384),
            "RS512" => Ok(Algorithm::RS512),
            "ES256" => Ok(Algorithm::ES256),
            "ES384" => Ok(Algorithm::ES384),
            _ => Err(JwkError::UnsupportedKeyType(format!(
                "Unsupported algorithm: {}",
                alg_str
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwks_parse_empty() {
        let json = r#"{"keys": []}"#;
        let jwks: Jwks = serde_json::from_str(json).unwrap();
        assert!(jwks.keys.is_empty());
    }

    #[test]
    fn test_jwk_key_is_rsa() {
        let key = JwkKey {
            kty: "RSA".to_string(),
            kid: Some("test".to_string()),
            key_use: None,
            alg: None,
            n: Some("test".to_string()),
            e: Some("AQAB".to_string()),
            crv: None,
            x: None,
            y: None,
        };
        assert!(key.is_rsa());
        assert!(!key.is_ec());
    }

    #[test]
    fn test_jwk_key_is_ec() {
        let key = JwkKey {
            kty: "EC".to_string(),
            kid: Some("test".to_string()),
            key_use: None,
            alg: None,
            n: None,
            e: None,
            crv: Some("P-256".to_string()),
            x: Some("test".to_string()),
            y: Some("test".to_string()),
        };
        assert!(key.is_ec());
        assert!(!key.is_rsa());
    }

    #[test]
    fn test_jwk_default_algorithm() {
        let rsa_key = JwkKey {
            kty: "RSA".to_string(),
            kid: None,
            key_use: None,
            alg: None,
            n: None,
            e: None,
            crv: None,
            x: None,
            y: None,
        };
        assert_eq!(rsa_key.algorithm(), Some("RS256"));

        let ec_key = JwkKey {
            kty: "EC".to_string(),
            kid: None,
            key_use: None,
            alg: None,
            n: None,
            e: None,
            crv: Some("P-256".to_string()),
            x: None,
            y: None,
        };
        assert_eq!(ec_key.algorithm(), Some("ES256"));
    }

    #[test]
    fn test_to_decoding_key_rsa_missing_n() {
        let key = JwkKey {
            kty: "RSA".to_string(),
            kid: Some("test".to_string()),
            key_use: None,
            alg: None,
            n: None, // missing
            e: Some("AQAB".to_string()),
            crv: None,
            x: None,
            y: None,
        };
        let result = key.to_decoding_key();
        assert!(result.is_err());
        match result.unwrap_err() {
            JwkError::MissingParameter(param) => {
                assert!(param.contains("modulus"));
            }
            _ => panic!("Expected MissingParameter error"),
        }
    }

    #[test]
    fn test_to_decoding_key_rsa_missing_e() {
        let key = JwkKey {
            kty: "RSA".to_string(),
            kid: Some("test".to_string()),
            key_use: None,
            alg: None,
            n: Some("test".to_string()),
            e: None, // missing
            crv: None,
            x: None,
            y: None,
        };
        let result = key.to_decoding_key();
        assert!(result.is_err());
        match result.unwrap_err() {
            JwkError::MissingParameter(param) => {
                assert!(param.contains("exponent"));
            }
            _ => panic!("Expected MissingParameter error"),
        }
    }

    #[test]
    fn test_to_decoding_key_ec_missing_x() {
        let key = JwkKey {
            kty: "EC".to_string(),
            kid: Some("test".to_string()),
            key_use: None,
            alg: None,
            n: None,
            e: None,
            crv: Some("P-256".to_string()),
            x: None, // missing
            y: Some("test".to_string()),
        };
        let result = key.to_decoding_key();
        assert!(result.is_err());
        match result.unwrap_err() {
            JwkError::MissingParameter(param) => {
                assert!(param.contains("x"));
            }
            _ => panic!("Expected MissingParameter error"),
        }
    }

    #[test]
    fn test_to_decoding_key_ec_missing_y() {
        let key = JwkKey {
            kty: "EC".to_string(),
            kid: Some("test".to_string()),
            key_use: None,
            alg: None,
            n: None,
            e: None,
            crv: Some("P-256".to_string()),
            x: Some("test".to_string()),
            y: None, // missing
        };
        let result = key.to_decoding_key();
        assert!(result.is_err());
        match result.unwrap_err() {
            JwkError::MissingParameter(param) => {
                assert!(param.contains("y"));
            }
            _ => panic!("Expected MissingParameter error"),
        }
    }

    #[test]
    fn test_to_decoding_key_unsupported_type() {
        let key = JwkKey {
            kty: "oct".to_string(), // symmetric key, not supported
            kid: Some("test".to_string()),
            key_use: None,
            alg: None,
            n: None,
            e: None,
            crv: None,
            x: None,
            y: None,
        };
        let result = key.to_decoding_key();
        assert!(result.is_err());
        match result.unwrap_err() {
            JwkError::UnsupportedKeyType(kty) => {
                assert_eq!(kty, "oct");
            }
            _ => panic!("Expected UnsupportedKeyType error"),
        }
    }

    #[test]
    fn test_jwt_algorithm_rsa() {
        let key = JwkKey {
            kty: "RSA".to_string(),
            kid: None,
            key_use: None,
            alg: Some("RS384".to_string()),
            n: None,
            e: None,
            crv: None,
            x: None,
            y: None,
        };
        let result = key.jwt_algorithm();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Algorithm::RS384);
    }

    #[test]
    fn test_jwt_algorithm_ec_p256() {
        let key = JwkKey {
            kty: "EC".to_string(),
            kid: None,
            key_use: None,
            alg: None, // should default to ES256 for P-256
            n: None,
            e: None,
            crv: Some("P-256".to_string()),
            x: None,
            y: None,
        };
        let result = key.jwt_algorithm();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Algorithm::ES256);
    }

    #[test]
    fn test_jwt_algorithm_ec_p384() {
        let key = JwkKey {
            kty: "EC".to_string(),
            kid: None,
            key_use: None,
            alg: None, // should default to ES384 for P-384
            n: None,
            e: None,
            crv: Some("P-384".to_string()),
            x: None,
            y: None,
        };
        let result = key.jwt_algorithm();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Algorithm::ES384);
    }
}
