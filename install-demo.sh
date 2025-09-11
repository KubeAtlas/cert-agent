#!/bin/bash

echo "🎯 Демонстрация установки cert-agent через apt"
echo "================================================"
echo ""

echo "📦 1. Добавление репозитория (симуляция)"
echo "   sudo apt-key adv --keyserver keyserver.ubuntu.com --recv-keys YOUR_KEY"
echo "   echo 'deb https://your-repo.com/debian stable main' | sudo tee /etc/apt/sources.list.d/cert-agent.list"
echo ""

echo "🔄 2. Обновление списка пакетов"
echo "   sudo apt update"
echo ""

echo "📥 3. Установка cert-agent"
echo "   sudo apt install cert-agent"
echo ""

echo "⚙️ 4. Во время установки debconf спросит:"
echo "   - Redis URL: redis://localhost:6380"
echo "   - gRPC порт: 50051"
echo "   - Включить сервис: Да"
echo ""

echo "🚀 5. После установки сервис автоматически:"
echo "   - Создаст пользователя cert-agent"
echo "   - Настроит systemd сервис"
echo "   - Запустит сервис"
echo "   - Настроит подключение к Redis"
echo ""

echo "✅ 6. Проверка работы:"
echo "   systemctl status cert-agent"
echo "   journalctl -u cert-agent -f"
echo ""

echo "🧪 7. Тестирование gRPC API:"
echo "   grpcurl -plaintext localhost:50051 list"
echo ""

echo "📁 8. Файлы конфигурации:"
echo "   /etc/cert-agent/config.toml"
echo "   /var/lib/cert-agent/"
echo "   /var/log/cert-agent/"
echo ""

echo "🎉 Установка завершена! cert-agent готов к работе!"
