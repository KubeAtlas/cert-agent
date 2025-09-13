# cert-agent

[![CI](https://github.com/your-org/cert-agent/workflows/CI/badge.svg)](https://github.com/your-org/cert-agent/actions)
[![Docker](https://github.com/your-org/cert-agent/workflows/Docker%20Build%20and%20Test/badge.svg)](https://github.com/your-org/cert-agent/actions)
[![Release](https://github.com/your-org/cert-agent/workflows/Build%20and%20Release%20Debian%20Packages/badge.svg)](https://github.com/your-org/cert-agent/releases)

Современный сервис для управления mTLS сертификатами с gRPC API, автоматическим обновлением и интеграцией с Redis.

## 🚀 Возможности

- **gRPC API** для управления сертификатами
- **Redis интеграция** для отслеживания сертификатов
- **Автоматическое обновление** сертификатов
- **Генерация mTLS сертификатов**
- **Поддержка отзыва сертификатов**
- **Мониторинг в реальном времени**
- **Debian пакеты** с debconf конфигурацией
- **Docker поддержка**

## 📦 Установка

### Через Debian пакеты (рекомендуется)

1. **Скачайте пакет для вашей архитектуры:**
   ```bash
   # Для AMD64
   wget https://github.com/your-org/cert-agent/releases/latest/download/cert-agent_*_amd64.deb
   
   # Для ARM64
   wget https://github.com/your-org/cert-agent/releases/latest/download/cert-agent_*_arm64.deb
   ```

2. **Установите пакет:**
   ```bash
   sudo dpkg -i cert-agent_*_amd64.deb
   sudo apt-get install -f  # Установите зависимости если нужно
   ```

3. **Настройте Redis подключение:**
   ```bash
   sudo dpkg-reconfigure cert-agent
   ```

### Через Docker

```bash
# Клонируйте репозиторий
git clone https://github.com/your-org/cert-agent.git
cd cert-agent

# Запустите с Docker Compose
docker-compose up -d

# Или соберите образ самостоятельно
docker build -t cert-agent .
docker run -d --name cert-agent -p 50051:50051 cert-agent
```

### Из исходного кода

```bash
# Установите Rust (https://rustup.rs/)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Клонируйте репозиторий
git clone https://github.com/your-org/cert-agent.git
cd cert-agent

# Соберите проект
cargo build --release

# Запустите сервис
./target/release/cert-agent --config config/default.toml
```

## ⚙️ Конфигурация

### Основные настройки

Файл конфигурации: `/etc/cert-agent/config.toml`

```toml
[grpc]
bind_address = "0.0.0.0:50051"
max_message_size = 4194304

[redis]
url = "redis://localhost:6379"
max_connections = 10
connection_timeout_secs = 5
command_timeout_secs = 3

[watcher]
check_interval_seconds = 300
renewal_threshold_days = 30
max_concurrent_renewals = 5

[certificate]
ca_cert_path = "/etc/cert-agent/ca.crt"
ca_key_path = "/etc/cert-agent/ca.key"
storage_path = "/var/lib/cert-agent/certs"
default_validity_days = 365
renewal_threshold_days = 30
key_size = 2048
signature_algorithm = "sha256"
```

### Переменные окружения

```bash
export CERT_AGENT_REDIS_URL="redis://localhost:6380"
export CERT_AGENT_GRPC_BIND_ADDRESS="0.0.0.0:50051"
export CERT_AGENT_CERTIFICATE_DEFAULT_VALIDITY_DAYS="365"
```

## 🔧 Использование

### gRPC API

#### Выпуск сертификата

```bash
grpcurl -plaintext -d '{
  "subject": "CN=example.com",
  "dns_names": ["example.com", "*.example.com"],
  "validity_days": 365
}' localhost:50051 cert_agent.CertAgent/IssueCertificate
```

#### Получение статуса сертификата

```bash
grpcurl -plaintext -d '{
  "certificate_id": "certificate-uuid"
}' localhost:50051 cert_agent.CertAgent/GetCertificateStatus
```

#### Список сертификатов

```bash
grpcurl -plaintext localhost:50051 cert_agent.CertAgent/ListCertificates
```

#### Отзыв сертификата

```bash
grpcurl -plaintext -d '{
  "certificate_id": "certificate-uuid"
}' localhost:50051 cert_agent.CertAgent/RevokeCertificate
```

### gRPC клиент

Для тестирования и интеграции используйте любой gRPC клиент:

```bash
# Используя grpcurl
grpcurl -plaintext -d '{"subject": "CN=example.com"}' \
  localhost:50051 cert_agent.CertAgent/IssueCertificate

# Используя собственный клиент на любом языке
# Примеры клиентов доступны в отдельном репозитории
```

## 🐳 Docker

### Docker Compose

```yaml
version: '3.8'

services:
  cert-agent:
    build: .
    ports:
      - "50051:50051"
    environment:
      - CERT_AGENT_REDIS_URL=redis://redis:6379
    depends_on:
      - redis
    volumes:
      - cert-data:/var/lib/cert-agent/certs

  redis:
    image: redis:7-alpine
    ports:
      - "6380:6379"
    volumes:
      - redis-data:/data

volumes:
  cert-data:
  redis-data:
```

### Docker образы

```bash
# Сборка образа
docker build -t cert-agent .

# Или использование готового образа
docker pull ghcr.io/YOUR_USERNAME/cert-agent:latest

# Запуск с Redis
docker run -d --name cert-agent \
  --link redis:redis \
  -p 50051:50051 \
  -e CERT_AGENT_REDIS_URL=redis://redis:6379 \
  ghcr.io/YOUR_USERNAME/cert-agent:latest
```

## 🏗️ Разработка

### Требования

- Rust 1.85+
- OpenSSL
- protobuf-compiler
- Redis (для тестирования)

### Сборка

```bash
# Установка зависимостей
sudo apt-get install libssl-dev pkg-config protobuf-compiler

# Сборка проекта
cargo build

# Запуск тестов
cargo test

# Проверка форматирования
cargo fmt --check

# Линтинг
cargo clippy
```

### Сборка пакетов

```bash
# Локальная сборка пакетов
./scripts/build-packages.sh 1.0.0

# Пакеты будут созданы в директории packages/
```

### Тестирование

```bash
# Запуск Redis для тестов
docker run -d --name test-redis -p 6379:6379 redis:7-alpine

# Запуск тестов
cargo test

# Интеграционные тесты
./scripts/test-integration.sh
```

## 📊 Мониторинг

### Логи

```bash
# Systemd сервис
journalctl -u cert-agent -f

# Docker
docker logs cert-agent -f
```

### Метрики

Сервис предоставляет метрики через gRPC и логи:

- Количество выпущенных сертификатов
- Количество обновленных сертификатов
- Количество отозванных сертификатов
- Статус подключения к Redis
- Время отклика gRPC запросов

### Health Check

```bash
# Проверка статуса сервиса
systemctl status cert-agent

# Проверка gRPC API
grpcurl -plaintext localhost:50051 grpc.health.v1.Health/Check
```

## 🔒 Безопасность

- Сертификаты хранятся в зашифрованном виде
- Приватные ключи защищены файловой системой
- gRPC соединения поддерживают TLS
- Redis соединения могут быть защищены паролем
- Системный пользователь для запуска сервиса

## 🤝 Вклад в проект

1. Форкните репозиторий
2. Создайте ветку для новой функции (`git checkout -b feature/amazing-feature`)
3. Зафиксируйте изменения (`git commit -m 'Add amazing feature'`)
4. Отправьте в ветку (`git push origin feature/amazing-feature`)
5. Откройте Pull Request

## 📄 Лицензия

Этот проект лицензирован под MIT License - см. файл [LICENSE](LICENSE) для деталей.

## 🆘 Поддержка

- 📖 [Документация](https://github.com/your-org/cert-agent/wiki)
- 🐛 [Баг-трекер](https://github.com/your-org/cert-agent/issues)
- 💬 [Discussions](https://github.com/your-org/cert-agent/discussions)

## 📈 Roadmap

- [ ] Web UI для управления сертификатами
- [ ] Поддержка ACME протокола
- [ ] Интеграция с HashiCorp Vault
- [ ] Метрики Prometheus
- [ ] Kubernetes оператор
- [ ] Поддержка Let's Encrypt
- [ ] Автоматическое обновление через webhook
 
  
   
   