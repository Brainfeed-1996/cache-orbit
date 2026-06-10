# Déploiement

## Démarrage local

```bash
make dev
```

## Docker Compose

```yaml
# cache-orbit local stack
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: cache_orbit
      POSTGRES_USER: orbit
      POSTGRES_PASSWORD: orbit
    volumes:
      - pgdata:/var/lib/postgresql/data
      - ./docker/postgres/init.sql:/docker-entrypoint-initdb.d/init.sql:ro
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U orbit"]
      interval: 5s
      timeout: 5s
      retries: 5

  nats:
    image: nats:2-alpine
    command: ["-js", "-m", "8222"]
    ports:
      - "4222:4222"
      - "8222:8222"
    healthcheck:
      test: ["CMD-SHELL", "nc -z localhost 4222"]
      interval: 5s

  control-plane:
    build:
      context: ./src/control-plane
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
      - "50051:50051"
    environment:
      DATABASE_URL: postgres://orbit:orbit@postgres:5432/cache_orbit?sslmode=disable
      NATS_URL: nats://nats:4222
    depends_on:
      postgres:
        condition: service_healthy
      nats:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:8080/healthz || exit 1"]
      interval: 5s

  cache-engine-1:
    build:
      context: .
      dockerfile: Dockerfile.cache-engine
    environment:
      NODE_ID: node-1
      LISTEN_ADDR: 0.0.0.0:6379
      DATACENTER: eu-west-1
      METRICS_ADDR: 0.0.0.0:9090
      CONTROL_PLANE: control-plane:50051
      PRIMARY_MAX_CAPACITY: 1000000
      L1_CAPACITY_MB: 256
      TTL_SECS: 300
    ports:
      - "16379:6379"
      - "19090:9090"
    depends_on:
      control-plane:
        condition: service_healthy

  cache-engine-2:
    build:
      context: .
      dockerfile: Dockerfile.cache-engine
    environment:
      NODE_ID: node-2
      LISTEN_ADDR: 0.0.0.0:6379
      DATACENTER: eu-west-1
      METRICS_ADDR: 0.0.0.0:9090
      CONTROL_PLANE: control-plane:50051
      PRIMARY_MAX_CAPACITY: 1000000
      L1_CAPACITY_MB: 256
      TTL_SECS: 300
    ports:
      - "16380:6379"
      - "19091:9090"
    depends_on:
      control-plane:
        condition: service_healthy

  cache-engine-3:
    build:
      context: .
      dockerfile: Dockerfile.cache-engine
    environment:
      NODE_ID: node-3
      LISTEN_ADDR: 0.0.0.0:6379
      DATACENTER: us-east-1
      METRICS_ADDR: 0.0.0.0:9090
      CONTROL_PLANE: control-plane:50051
      PRIMARY_MAX_CAPACITY: 1000000
      L1_CAPACITY_MB: 256
      TTL_SECS: 300
    ports:
      - "16381:6379"
      - "19092:9090"
    depends_on:
      control-plane:
        condition: service_healthy

  cache-engine-4:
    build:
      context: .
      dockerfile: Dockerfile.cache-engine
    environment:
      NODE_ID: node-4
      LISTEN_ADDR: 0.0.0.0:6379
      DATACENTER: us-east-1
      METRICS_ADDR: 0.0.0.0:9090
      CONTROL_PLANE: control-plane:50051
      PRIMARY_MAX_CAPACITY: 1000000
      L1_CAPACITY_MB: 256
      TTL_SECS: 300
    ports:
      - "16382:6379"
      - "19093:9090"
    depends_on:
      control-plane:
        condition: service_healthy

  operator-ui:
    build:
      context: ./src/operator-ui
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      NEXT_PUBLIC_API_URL: http://control-plane:8080
      NEXT_PUBLIC_WS_URL: ws://control-plane:8080
    depends_on:
      control-plane:
        condition: service_healthy

  prometheus:
    image: prom/prometheus
    volumes:
      - ./docker/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
    ports:
      - "9090:9090"
    depends_on:
      - cache-engine-1
      - cache-engine-2

  grafana:
    image: grafana/grafana
    ports:
      - "3001:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: cache-orbit
    volumes:
      - grafana_data:/var/lib/grafana
      - ./docker/grafana/provisioning:/etc/grafana/provisioning:ro

  redis-streams:
    image: redis:7-alpine
    command: ["redis-server", "--appendonly", "yes"]
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

volumes:
  pgdata:
  grafana_data:
  redis_data:
```

## Variables d'environnement critiques

| Variable | Défaut | Description |
|---------|--------|-------------|
| NODE_ID | `node-1` | Identifiant unique du nœud de cache |
| DATACENTER | `eu-west-1` | Datacenter du nœud |
| LISTEN_ADDR | `0.0.0.0:6379` | Adresse d'écoute RESP |
| METRICS_ADDR | `0.0.0.0:9090` | Adresse d'écoute Prometheus |
| CONTROL_PLANE | `control-plane:50051` | Adresse gRPC du control plane |
| PRIMARY_MAX_CAPACITY | `1000000` | Capacité max du cache primaire (entrées) |
| L1_CAPACITY_MB | `256` | Taille du cache local par nœud |
| TTL_SECS | `300` | TTL global par défaut |

## Production

### Kubernetes

- Chaque nœud cache est déployé en `StatefulSet` (identité réseau persistante).
- Le **control plane** tourne avec 3 réplicas derrière un ClusterIP.
- Topologie sauvegardée dans PostgreSQL haute-disponibilité (Patroni).
- NATS Cluster en mode JetStream pour la résilience.

### Sécurité

- mTLS obligatoire inter-services,
- RBAC via OPA (Open Policy Agent),
- rotation des secrets via Kubernetes Secrets + Vault.

### Scaling

- **Horizontal** : ajouter un nœud → mise à jour topologie → rééquilibrage des partitions.
- **Vertical** : augmenter `PRIMARY_MAX_CAPACITY` → redémarrage sans migration de données (les entrées sont les mêmes).
