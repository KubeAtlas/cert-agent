.PHONY: help build run test clean docker-build docker-up docker-down client-build client-run

# Default target
help:
	@echo "Доступные команды:"
	@echo "  build          - Сборка основного сервиса"
	@echo "  run            - Запуск сервиса локально"
	@echo "  test           - Запуск тестов"
	@echo "  clean          - Очистка build артефактов"
	@echo "  docker-build   - Сборка Docker образа"
	@echo "  docker-up      - Запуск в Docker Compose"
	@echo "  docker-down    - Остановка Docker Compose"
	@echo "  docker-logs    - Просмотр логов Docker"
	@echo "  client-build   - Сборка gRPC клиента"
	@echo "  client-run     - Запуск gRPC клиента для тестирования"
	@echo "  grpcui         - Запуск gRPC UI для тестирования"

# Сборка основного сервиса
build:
	cargo build --release

# Запуск сервиса локально
run:
	cargo run

# Запуск тестов
test:
	cargo test

# Очистка
clean:
	cargo clean

# Сборка Docker образа
docker-build:
	docker build -t cert-agent:latest .

# Запуск в Docker Compose
docker-up:
	docker-compose up -d
	@echo "Сервисы запущены:"
	@echo "  - cert-agent: http://localhost:50051"
	@echo "  - Redis: localhost:6380"
	@echo "  - gRPC UI: http://localhost:8080 (если запущен с --profile tools)"

# Остановка Docker Compose
docker-down:
	docker-compose down

# Просмотр логов
docker-logs:
	docker-compose logs -f

# Сборка gRPC клиента
client-build:
	cd client && cargo build --release

# Запуск gRPC клиента
client-run:
	cd client && cargo run

# Запуск gRPC UI
grpcui:
	docker-compose --profile tools up -d grpc-client
	@echo "gRPC UI доступен по адресу: http://localhost:8080"

# Полная очистка (включая Docker)
clean-all: clean docker-down
	docker system prune -f
	docker volume prune -f

# Проверка состояния сервисов
status:
	@echo "=== Docker Compose статус ==="
	docker-compose ps
	@echo ""
	@echo "=== Проверка подключения к Redis ==="
	@docker exec cert-agent-redis redis-cli ping || echo "Redis недоступен"
	@echo ""
	@echo "=== Проверка gRPC сервиса ==="
	@nc -z localhost 50051 && echo "gRPC сервис доступен" || echo "gRPC сервис недоступен"

# Debian package commands
deb-build:
	@echo "🔨 Сборка Debian пакета..."
	dpkg-buildpackage -us -uc -b
	@echo "✅ Debian пакет собран!"

deb-clean:
	@echo "🧹 Очистка файлов сборки..."
	debian/rules clean
	rm -f ../cert-agent_*.deb ../cert-agent_*.changes ../cert-agent_*.buildinfo
	@echo "✅ Очистка завершена!"

deb-install:
	@echo "📦 Установка Debian пакета..."
	sudo dpkg -i ../cert-agent_*.deb || sudo apt-get install -f
	@echo "✅ Пакет установлен!"

deb-remove:
	@echo "🗑️ Удаление Debian пакета..."
	sudo apt-get remove --purge cert-agent
	@echo "✅ Пакет удален!"

# Полная сборка и тестирование пакета
deb-test: deb-build deb-install
	@echo "🧪 Тестирование установленного пакета..."
	systemctl status cert-agent
	@echo "✅ Тестирование завершено!"
