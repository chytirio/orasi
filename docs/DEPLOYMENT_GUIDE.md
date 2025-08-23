# Orasi Ingestion Deployment Guide

## Overview

This guide provides comprehensive instructions for deploying the Orasi Ingestion platform in various environments, from development to production.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Development Setup](#development-setup)
3. [Docker Deployment](#docker-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Production Deployment](#production-deployment)
6. [Configuration Management](#configuration-management)
7. [Monitoring Setup](#monitoring-setup)
8. [Security Configuration](#security-configuration)
9. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

- **CPU**: 2+ cores (4+ recommended for production)
- **Memory**: 4GB+ RAM (8GB+ recommended for production)
- **Storage**: 10GB+ available disk space
- **Network**: Stable internet connection for dependencies

### Software Requirements

- **Rust**: 1.70+ (for building from source)
- **Docker**: 20.10+ (for containerized deployment)
- **Kubernetes**: 1.24+ (for K8s deployment)
- **Helm**: 3.8+ (for K8s package management)

### Dependencies

- **Kafka**: 2.8+ (if using Kafka protocol)
- **Jaeger**: 1.40+ (for distributed tracing)
- **Prometheus**: 2.40+ (for metrics collection)
- **Grafana**: 9.0+ (for metrics visualization)

## Development Setup

### Local Development

1. **Clone the repository**

```bash
git clone https://github.com/chytirio/orasi.git
cd orasi
```

2. **Install Rust dependencies**

```bash
cargo build --release
```

3. **Create configuration file**

```bash
cp config/bridge.toml.example config/bridge.toml
```

4. **Edit configuration**

```toml
[ingestion]
enabled = true
max_batch_size = 1000

[ingestion.protocols.otlp]
enabled = true
host = "127.0.0.1"
port = 4318

[monitoring]
enabled = true

[monitoring.metrics]
enabled = true
port = 9090
```

5. **Run the application**

```bash
cargo run --bin ingestion
```

### Development with Docker Compose

1. **Create docker-compose.yml**

```yaml
version: '3.8'

services:
  ingestion:
    build: .
    ports:
      - "4318:4318"  # OTLP HTTP
      - "4317:4317"  # OTLP gRPC
      - "9090:9090"  # Metrics
    volumes:
      - ./config:/app/config
    environment:
      - RUST_LOG=info
    depends_on:
      - kafka
      - jaeger
      - prometheus

  kafka:
    image: confluentinc/cp-kafka:7.3.0
    ports:
      - "9092:9092"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka:29092,PLAINTEXT_HOST://localhost:9092
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
      KAFKA_TRANSACTION_STATE_LOG_MIN_ISR: 1
      KAFKA_TRANSACTION_STATE_LOG_REPLICATION_FACTOR: 1

  zookeeper:
    image: confluentinc/cp-zookeeper:7.3.0
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000

  jaeger:
    image: jaegertracing/all-in-one:1.40
    ports:
      - "16686:16686"
      - "14268:14268"
    environment:
      - COLLECTOR_OTLP_ENABLED=true

  prometheus:
    image: prom/prometheus:v2.40.0
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'

  grafana:
    image: grafana/grafana:9.0.0
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-storage:/var/lib/grafana

volumes:
  grafana-storage:
```

2. **Create prometheus.yml**

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'ingestion'
    static_configs:
      - targets: ['ingestion:9090']
```

3. **Start services**

```bash
docker-compose up -d
```

## Docker Deployment

### Building the Image

1. **Create Dockerfile**

```dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .

RUN cargo build --release --bin ingestion

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/ingestion /app/ingestion
COPY --from=builder /app/config /app/config

EXPOSE 4318 4317 9090

CMD ["/app/ingestion"]
```

2. **Build image**

```bash
docker build -t orasi/ingestion:latest .
```

### Running with Docker

1. **Basic run**

```bash
docker run -d \
  --name ingestion \
  -p 4318:4318 \
  -p 4317:4317 \
  -p 9090:9090 \
  -v $(pwd)/config:/app/config \
  orasi/ingestion:latest
```

2. **With environment variables**

```bash
docker run -d \
  --name ingestion \
  -p 4318:4318 \
  -p 4317:4317 \
  -p 9090:9090 \
  -e ORASI_INGESTION_ENABLED=true \
  -e ORASI_MONITORING_ENABLED=true \
  -e ORASI_LOG_LEVEL=info \
  orasi/ingestion:latest
```

## Kubernetes Deployment

### Using Helm

1. **Create Helm chart structure**

```
helm/
├── Chart.yaml
├── values.yaml
├── templates/
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   └── ingress.yaml
└── charts/
```

2. **Chart.yaml**

```yaml
apiVersion: v2
name: orasi-ingestion
description: Orasi Telemetry Ingestion Platform
version: 0.1.0
appVersion: "0.1.0"
```

3. **values.yaml**

```yaml
replicaCount: 3

image:
  repository: orasi/ingestion
  tag: latest
  pullPolicy: IfNotPresent

service:
  type: ClusterIP
  otlpHttpPort: 4318
  otlpGrpcPort: 4317
  metricsPort: 9090

ingress:
  enabled: true
  className: nginx
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
  hosts:
    - host: ingestion.example.com
      paths:
        - path: /
          pathType: Prefix

resources:
  limits:
    cpu: 1000m
    memory: 2Gi
  requests:
    cpu: 500m
    memory: 1Gi

config:
  ingestion:
    enabled: true
    max_batch_size: 1000
  
  monitoring:
    enabled: true
    
  security:
    enabled: true
```

4. **deployment.yaml**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "orasi-ingestion.fullname" . }}
  labels:
    {{- include "orasi-ingestion.labels" . | nindent 4 }}
spec:
  replicas: {{ .Values.replicaCount }}
  selector:
    matchLabels:
      {{- include "orasi-ingestion.selectorLabels" . | nindent 6 }}
  template:
    metadata:
      labels:
        {{- include "orasi-ingestion.selectorLabels" . | nindent 8 }}
    spec:
      containers:
        - name: {{ .Chart.Name }}
          image: "{{ .Values.image.repository }}:{{ .Values.image.tag }}"
          imagePullPolicy: {{ .Values.image.pullPolicy }}
          ports:
            - name: otlp-http
              containerPort: {{ .Values.service.otlpHttpPort }}
            - name: otlp-grpc
              containerPort: {{ .Values.service.otlpGrpcPort }}
            - name: metrics
              containerPort: {{ .Values.service.metricsPort }}
          env:
            - name: ORASI_INGESTION_ENABLED
              value: "{{ .Values.config.ingestion.enabled }}"
            - name: ORASI_MONITORING_ENABLED
              value: "{{ .Values.config.monitoring.enabled }}"
            - name: ORASI_SECURITY_ENABLED
              value: "{{ .Values.config.security.enabled }}"
          resources:
            {{- toYaml .Values.resources | nindent 12 }}
          livenessProbe:
            httpGet:
              path: /health
              port: metrics
            initialDelaySeconds: 30
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /ready
              port: metrics
            initialDelaySeconds: 5
            periodSeconds: 5
```

5. **Install with Helm**

```bash
# Add Helm repository
helm repo add orasi https://charts.orasi.io
helm repo update

# Install chart
helm install ingestion orasi/orasi-ingestion \
  --namespace monitoring \
  --create-namespace \
  --values values.yaml
```

### Manual Kubernetes Deployment

1. **Create namespace**

```bash
kubectl create namespace monitoring
```

2. **Create ConfigMap**

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: ingestion-config
  namespace: monitoring
data:
  bridge.toml: |
    [ingestion]
    enabled = true
    max_batch_size = 1000
    
    [ingestion.protocols.otlp]
    enabled = true
    host = "0.0.0.0"
    port = 4318
    
    [monitoring]
    enabled = true
    
    [monitoring.metrics]
    enabled = true
    port = 9090
```

3. **Create Deployment**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ingestion
  namespace: monitoring
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ingestion
  template:
    metadata:
      labels:
        app: ingestion
    spec:
      containers:
      - name: ingestion
        image: orasi/ingestion:latest
        ports:
        - containerPort: 4318
          name: otlp-http
        - containerPort: 4317
          name: otlp-grpc
        - containerPort: 9090
          name: metrics
        volumeMounts:
        - name: config
          mountPath: /app/config
        livenessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 9090
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: ingestion-config
```

4. **Create Service**

```yaml
apiVersion: v1
kind: Service
metadata:
  name: ingestion
  namespace: monitoring
spec:
  selector:
    app: ingestion
  ports:
  - name: otlp-http
    port: 4318
    targetPort: 4318
  - name: otlp-grpc
    port: 4317
    targetPort: 4317
  - name: metrics
    port: 9090
    targetPort: 9090
  type: ClusterIP
```

5. **Apply manifests**

```bash
kubectl apply -f configmap.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
```

## Production Deployment

### High Availability Setup

1. **Multi-zone deployment**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ingestion
spec:
  replicas: 6
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  template:
    spec:
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - ingestion
              topologyKey: kubernetes.io/hostname
        nodeAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            nodeSelectorTerms:
            - matchExpressions:
              - key: node-role.kubernetes.io/worker
                operator: Exists
```

2. **Horizontal Pod Autoscaler**

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: ingestion-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: ingestion
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Load Balancing

1. **Ingress configuration**

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: ingestion-ingress
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
  - hosts:
    - ingestion.example.com
    secretName: ingestion-tls
  rules:
  - host: ingestion.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: ingestion
            port:
              number: 4318
```

### Backup and Recovery

1. **Persistent Volume for configuration**

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: ingestion-config-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
  storageClassName: fast-ssd
```

2. **Backup job**

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: ingestion-backup
spec:
  schedule: "0 2 * * *"
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: alpine:latest
            command:
            - /bin/sh
            - -c
            - |
              kubectl get configmap ingestion-config -o yaml > /backup/config-$(date +%Y%m%d).yaml
              kubectl get secret ingestion-secrets -o yaml > /backup/secrets-$(date +%Y%m%d).yaml
          volumes:
          - name: backup-storage
            persistentVolumeClaim:
              claimName: backup-pvc
          restartPolicy: OnFailure
```

## Configuration Management

### Environment-Specific Configuration

1. **Development configuration**

```toml
[ingestion]
enabled = true
max_batch_size = 100

[monitoring]
enabled = true
level = "debug"

[security]
enabled = false
```

2. **Staging configuration**

```toml
[ingestion]
enabled = true
max_batch_size = 500

[monitoring]
enabled = true
level = "info"

[security]
enabled = true
auth_method = "jwt"
```

3. **Production configuration**

```toml
[ingestion]
enabled = true
max_batch_size = 1000

[monitoring]
enabled = true
level = "warn"

[security]
enabled = true
auth_method = "jwt"
encryption_enabled = true
audit_logging = true
```

### Configuration Validation

1. **Validation script**

```bash
#!/bin/bash

# Validate configuration
cargo run --bin config-validator -- --config config/bridge.toml

# Check for required environment variables
required_vars=("ORASI_ENVIRONMENT" "ORASI_LOG_LEVEL")
for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo "Error: Required environment variable $var is not set"
        exit 1
    fi
done
```

## Monitoring Setup

### Prometheus Configuration

1. **Prometheus scrape config**

```yaml
scrape_configs:
  - job_name: 'orasi-ingestion'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        regex: ingestion
        action: keep
      - source_labels: [__meta_kubernetes_pod_container_port_number]
        regex: "9090"
        action: keep
    metrics_path: /metrics
    scrape_interval: 15s
```

2. **Alerting rules**

```yaml
groups:
  - name: orasi-ingestion
    rules:
      - alert: IngestionHighErrorRate
        expr: rate(ingestion_errors_total[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High error rate in ingestion"
          description: "Error rate is {{ $value }} errors per second"

      - alert: IngestionHighLatency
        expr: histogram_quantile(0.95, rate(ingestion_processing_duration_seconds_bucket[5m])) > 1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High latency in ingestion"
          description: "95th percentile latency is {{ $value }} seconds"
```

### Grafana Dashboards

1. **Ingestion Overview Dashboard**

```json
{
  "dashboard": {
    "title": "Orasi Ingestion Overview",
    "panels": [
      {
        "title": "Records Processed",
        "type": "stat",
        "targets": [
          {
            "expr": "rate(ingestion_records_total[5m])",
            "legendFormat": "{{protocol}}"
          }
        ]
      },
      {
        "title": "Processing Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(ingestion_processing_duration_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          }
        ]
      }
    ]
  }
}
```

## Security Configuration

### Authentication Setup

1. **JWT configuration**

```toml
[security.auth]
enabled = true
method = "jwt"
jwt_secret = "your-secret-key"
jwt_expiration = 3600
```

2. **OAuth2 configuration**

```toml
[security.auth]
enabled = true
method = "oauth2"
oauth2_provider = "google"
client_id = "your-client-id"
client_secret = "your-client-secret"
redirect_uri = "https://ingestion.example.com/auth/callback"
```

### Authorization Setup

1. **RBAC configuration**

```toml
[security.authorization]
enabled = true
default_role = "user"
enable_audit_log = true
max_audit_entries = 10000
audit_retention_days = 90

[[security.authorization.roles]]
name = "admin"
description = "Administrator with full access"
permissions = ["*"]

[[security.authorization.roles]]
name = "user"
description = "Standard user"
permissions = ["ingestion:read", "ingestion:write"]

[[security.authorization.roles]]
name = "readonly"
description = "Read-only access"
permissions = ["ingestion:read"]
```

### TLS/SSL Configuration

1. **TLS certificate setup**

```bash
# Generate self-signed certificate for development
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Create Kubernetes secret
kubectl create secret tls ingestion-tls \
  --cert=cert.pem \
  --key=key.pem \
  --namespace=monitoring
```

2. **TLS configuration**

```toml
[security.tls]
enabled = true
cert_file = "/etc/ssl/certs/ingestion.crt"
key_file = "/etc/ssl/private/ingestion.key"
ca_file = "/etc/ssl/certs/ca.crt"
```

## Troubleshooting

### Common Issues

1. **High memory usage**

```bash
# Check memory usage
kubectl top pods -l app=ingestion

# Check for memory leaks
kubectl logs ingestion-pod --previous | grep "memory"

# Adjust batch size
kubectl patch configmap ingestion-config --patch '{"data":{"bridge.toml":"[ingestion]\nmax_batch_size = 500"}}'
```

2. **High CPU usage**

```bash
# Check CPU usage
kubectl top pods -l app=ingestion

# Check for CPU-intensive operations
kubectl logs ingestion-pod | grep "processing"

# Scale up if needed
kubectl scale deployment ingestion --replicas=6
```

3. **Network connectivity issues**

```bash
# Check service endpoints
kubectl get endpoints ingestion

# Test connectivity
kubectl run test-pod --image=busybox --rm -it --restart=Never -- wget -O- http://ingestion:9090/health

# Check DNS resolution
kubectl run test-pod --image=busybox --rm -it --restart=Never -- nslookup ingestion
```

### Debugging

1. **Enable debug logging**

```bash
# Update log level
kubectl patch configmap ingestion-config --patch '{"data":{"bridge.toml":"[monitoring]\nlevel = \"debug\""}}'

# Restart pods
kubectl rollout restart deployment ingestion
```

2. **Check logs**

```bash
# Get logs from all pods
kubectl logs -l app=ingestion --tail=100

# Follow logs
kubectl logs -l app=ingestion -f

# Get logs from specific pod
kubectl logs ingestion-pod-name --tail=100
```

3. **Check metrics**

```bash
# Port forward to access metrics
kubectl port-forward svc/ingestion 9090:9090

# Check metrics endpoint
curl http://localhost:9090/metrics
```

### Performance Tuning

1. **Optimize batch processing**

```toml
[ingestion]
max_batch_size = 2000
max_wait_time = "10s"

[ingestion.processors.batch]
max_batch_size = 2000
max_wait_time = "10s"
```

2. **Optimize memory usage**

```toml
[ingestion]
buffer_size = 10000
max_memory_usage = "2GB"

[monitoring]
metrics_buffer_size = 1000
```

3. **Optimize network**

```toml
[ingestion.protocols.otlp]
max_concurrent_requests = 100
request_timeout = "30s"

[ingestion.exporters.lakehouse]
connection_pool_size = 10
max_retries = 3
```

### Health Checks

1. **Liveness probe**

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 9090
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3
```

2. **Readiness probe**

```yaml
readinessProbe:
  httpGet:
    path: /ready
    port: 9090
  initialDelaySeconds: 5
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 3
```

### Monitoring and Alerting

1. **Set up alerts**

```yaml
# Prometheus alerting rules
groups:
  - name: orasi-ingestion
    rules:
      - alert: IngestionDown
        expr: up{job="orasi-ingestion"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Ingestion service is down"
```

2. **Monitor key metrics**

- Records processed per second
- Processing latency (95th percentile)
- Error rate
- Memory usage
- CPU usage
- Network I/O

### Backup and Recovery

1. **Configuration backup**

```bash
#!/bin/bash
# Backup configuration
kubectl get configmap ingestion-config -o yaml > backup/config-$(date +%Y%m%d).yaml
kubectl get secret ingestion-secrets -o yaml > backup/secrets-$(date +%Y%m%d).yaml
```

2. **Recovery procedure**

```bash
#!/bin/bash
# Restore configuration
kubectl apply -f backup/config-20240127.yaml
kubectl apply -f backup/secrets-20240127.yaml

# Restart deployment
kubectl rollout restart deployment ingestion
```

## Support

For additional support:

1. **Documentation**: Check the full documentation
2. **Issues**: Report issues on GitHub
3. **Community**: Join the community discussions
4. **Professional Support**: Contact for enterprise support
