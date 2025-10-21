// Configuration module

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub buckets: Vec<BucketConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    pub path_prefix: String,
    pub s3: S3Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_empty_config_struct() {
        let _config = Config {
            server: ServerConfig {
                address: String::from("127.0.0.1"),
                port: 8080,
            },
            buckets: vec![],
        };
    }

    #[test]
    fn test_can_deserialize_minimal_valid_yaml_config() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        // If we got here, deserialization succeeded
        let _ = config;
    }

    #[test]
    fn test_can_access_server_address_from_config() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        assert_eq!(config.server.address, "127.0.0.1");
    }

    #[test]
    fn test_can_access_server_port_from_config() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        assert_eq!(config.server.port, 8080);
    }

    #[test]
    fn test_config_deserialization_fails_with_empty_file() {
        let yaml = "";
        let result: Result<Config, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "Expected deserialization to fail with empty file"
        );
    }

    #[test]
    fn test_config_deserialization_fails_with_invalid_yaml() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: [invalid syntax here}
"#;
        let result: Result<Config, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "Expected deserialization to fail with invalid YAML"
        );
    }

    #[test]
    fn test_can_parse_single_bucket_configuration() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        assert_eq!(config.buckets.len(), 1);
        assert_eq!(config.buckets[0].name, "products");
        assert_eq!(config.buckets[0].path_prefix, "/products");
        assert_eq!(config.buckets[0].s3.bucket, "my-products-bucket");
        assert_eq!(config.buckets[0].s3.region, "us-west-2");
    }

    #[test]
    fn test_can_parse_multiple_bucket_configurations() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  - name: "images"
    path_prefix: "/images"
    s3:
      bucket: "my-images-bucket"
      region: "us-east-1"
      access_key: "AKIAIOSFODNN7EXAMPLE2"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        assert_eq!(config.buckets.len(), 2);
        assert_eq!(config.buckets[0].name, "products");
        assert_eq!(config.buckets[1].name, "images");
        assert_eq!(config.buckets[0].path_prefix, "/products");
        assert_eq!(config.buckets[1].path_prefix, "/images");
    }

    #[test]
    fn test_can_access_bucket_name_from_config() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        assert_eq!(config.buckets[0].name, "products");
    }

    #[test]
    fn test_can_access_bucket_path_prefix_from_config() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        assert_eq!(config.buckets[0].path_prefix, "/products");
    }

    #[test]
    fn test_can_access_s3_bucket_name_from_config() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        assert_eq!(config.buckets[0].s3.bucket, "my-products-bucket");
    }

    #[test]
    fn test_can_access_s3_region_from_config() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
        let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
        assert_eq!(config.buckets[0].s3.region, "us-west-2");
    }
}
