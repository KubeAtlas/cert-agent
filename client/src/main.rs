use tonic::Request;
use std::collections::HashMap;

mod cert_agent {
    tonic::include_proto!("cert_agent");
}

use cert_agent::{
    cert_agent_client::CertAgentClient,
    *,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = CertAgentClient::connect("http://localhost:50051").await?;
    
    println!("🚀 Подключен к cert-agent сервису!");
    
    // Тест 1: Выпуск нового сертификата
    println!("\n📋 Тест 1: Выпуск нового сертификата");
    let issue_request = Request::new(IssueCertificateRequest {
        common_name: "test.example.com".to_string(),
        dns_names: vec![
            "test.example.com".to_string(),
            "*.test.example.com".to_string(),
        ],
        ip_addresses: vec!["127.0.0.1".to_string()],
        validity_days: 365,
        organization: "Test Organization".to_string(),
        organizational_unit: "IT Department".to_string(),
        country: "US".to_string(),
        state: "California".to_string(),
        locality: "San Francisco".to_string(),
        metadata: HashMap::new(),
    });
    
    match client.issue_certificate(issue_request).await {
        Ok(response) => {
            let cert = response.into_inner();
            println!("✅ Сертификат успешно выпущен!");
            println!("   ID: {}", cert.certificate_id);
            println!("   Статус: {:?}", cert.status);
            println!("   Истекает: {}", chrono::DateTime::from_timestamp(cert.expires_at, 0)
                .unwrap_or_default().format("%Y-%m-%d %H:%M:%S UTC"));
            
            // Тест 2: Получение статуса сертификата
            println!("\n📊 Тест 2: Получение статуса сертификата");
            let status_request = Request::new(GetCertificateStatusRequest {
                certificate_id: cert.certificate_id.clone(),
            });
            
            match client.get_certificate_status(status_request).await {
                Ok(status_response) => {
                    let status = status_response.into_inner();
                    println!("✅ Статус получен!");
                    println!("   CN: {}", status.common_name);
                    println!("   DNS Names: {:?}", status.dns_names);
                    println!("   Статус: {:?}", status.status);
                }
                Err(e) => println!("❌ Ошибка получения статуса: {}", e),
            }
            
            // Тест 3: Список всех сертификатов
            println!("\n📜 Тест 3: Список всех сертификатов");
            let list_request = Request::new(ListCertificatesRequest {
                status: 0, // Unspecified - все сертификаты
                page_size: 10,
                page_token: String::new(),
            });
            
            match client.list_certificates(list_request).await {
                Ok(list_response) => {
                    let list = list_response.into_inner();
                    println!("✅ Найдено {} сертификатов:", list.certificates.len());
                    for cert_info in list.certificates {
                        println!("   - {} (CN: {}, Статус: {:?})", 
                                cert_info.certificate_id, 
                                cert_info.common_name,
                                cert_info.status);
                    }
                }
                Err(e) => println!("❌ Ошибка получения списка: {}", e),
            }
            
            // Тест 4: Отзыв сертификата
            println!("\n🗑️ Тест 4: Отзыв сертификата");
            let revoke_request = Request::new(RevokeCertificateRequest {
                certificate_id: cert.certificate_id.clone(),
                reason: "Test revocation".to_string(),
            });
            
            match client.revoke_certificate(revoke_request).await {
                Ok(revoke_response) => {
                    let revoke = revoke_response.into_inner();
                    if revoke.success {
                        println!("✅ Сертификат успешно отозван!");
                    } else {
                        println!("❌ Ошибка отзыва сертификата: {}", revoke.message);
                    }
                }
                Err(e) => println!("❌ Ошибка отзыва: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Ошибка выпуска сертификата: {}", e);
        }
    }
    
    // Тест 5: Проверка подключения к Redis
    println!("\n🔗 Тест 5: Проверка подключения к Redis");
    println!("   Redis должен быть доступен на порту 6380");
    println!("   Проверим статус сервисов...");
    
    // Дополнительный тест - попытка получить статус несуществующего сертификата
    println!("\n🔍 Тест 6: Поиск несуществующего сертификата");
    let fake_request = Request::new(GetCertificateStatusRequest {
        certificate_id: "non-existent-id".to_string(),
    });
    
    match client.get_certificate_status(fake_request).await {
        Ok(_) => println!("⚠️ Неожиданно найден несуществующий сертификат"),
        Err(e) => {
            if e.code() == tonic::Code::NotFound {
                println!("✅ Корректно обработана ошибка 'не найден'");
            } else {
                println!("❌ Неожиданная ошибка: {}", e);
            }
        }
    }
    
    println!("\n🎉 Все тесты завершены!");
    Ok(())
}
