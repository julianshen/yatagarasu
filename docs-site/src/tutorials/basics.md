# Basic Setup

## Configuration File

Yatagarasu uses a YAML configuration file. By default, it looks for `config.yaml` in the current directory or `/etc/yatagarasu/config.yaml`.

## Server Configuration

```yaml
server:
  # The address to bind to
  addr: "0.0.0.0:8080"
```

## Bucket Configuration

You can define multiple S3 buckets to proxy.

```yaml
buckets:
  - name: "my-bucket"
    region: "us-east-1"
    endpoint: "s3.amazonaws.com" # Optional, defaults to AWS
    access_key: "AWS_ACCESS_KEY"
    secret_key: "AWS_SECRET_KEY"
    
  - name: "local-minio"
    region: "us-east-1"
    endpoint: "http://minio:9000"
    access_key: "minioadmin"
    secret_key: "minioadmin"
```

With this config, a request to `http://localhost:8080/my-bucket/image.jpg` will be proxied to AWS S3, and `http://localhost:8080/local-minio/doc.pdf` will go to your MinIO instance.
