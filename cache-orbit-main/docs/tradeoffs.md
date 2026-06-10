# Trade-offs

## Fraîcheur vs Latence vs Hit ratio

Une politique de fraîcheur agressive réduit la probabilité de stale reads mais augmente la pression backend (moins de hits) et ajoute de la latence (vérification de version, rechargement).

Choix de Cache Orbit : **exposition explicite de la sémantique de cohérence au développeur**. Le système supporte 3 niveaux (`Weak`, `BoundedStaleness`, `Strong`) mais ne masque pas le compromis.

## Richesse de l'invalidation vs Complexité

Un système qui invalide par clé, par pattern, par tag et par dépendance est plus correct, mais :

- la fan-out d'invalidation augmente avec le cardinal de tags,
- la compensation cross-shard devient critique.

Choix : **Canal dédié d'invalidation (NATS/Redis Streams)** avec at-least-once et idempotence par `(key, scope, version)`.

## Moteur custom vs Wrapper Redis

Un wrapper Redis permet une migration rapide mais réduit le contrôle sur :

- la sémantique multi-shard interne,
- l'embeddability (LIC, hot-key local),
- le typage et l'observabilité end-to-end.

Choix : **implémentation Rust custom** (filtrage, cache local, résilience). Le protocole reste compatible protocole RESP pour faciliter l'intégration avec des clients Redis existants.

## Passerelle Go vs pure Rust

Une API conçue en Rust évite un service supplémentaire et garantit zéro-copie potentiel. Toutefois l'écosystème **Go** est plus mature autour des contrôleurs cloud-native (operators, K8s CRDs, génération automatique de docs gRPC).

Choix : **Go pour le control-plane** (codomain-specific) et **Rust pour le data-plane** (performance critique). La communication inter-services passe par gRPC fortement typé.

## Connaissances non exploitées

- **Déduplication par bloom filter** pour éviter le ping des shards : écart car la cardinalité du keyspace est connue (1024 partitions), pas de problème caché.
- **WASM Runtime** pour exécuter les *invoke hooks* côté nœud : abandonné pour cause de complexité opérationnelle et surface d'attaque.
- **Read-your-writes via horloge vectorielle** : trop coûteux en bande passante pour des lectures user-facing, décomposé en read-after-write explicite via API.
