use super::*;
use std::collections::HashMap;

#[tokio::test]
async fn test_security_manager_integration() {
    // Create test configurations
    let auth_config = AuthConfig {
        method: AuthMethod::None,
        jwt_secret: Some("test-secret".to_string()),
        users: HashMap::new(),
    };
    
    let authorization_config = AuthorizationConfig {
        roles: HashMap::new(),
        user_roles: HashMap::new(),
        audit_logging: true,
    };
    
    let tls_config = TlsConfig::default();
    
    // Create security manager
    let security_manager = SecurityManager::new(
        auth_config,
        authorization_config,
        Some(tls_config)
    ).await.unwrap();
    
    // Test authentication
    let auth_result = security_manager.authenticate("test-user", "test-password").await;
    assert!(auth_result.is_ok());
    
    // Test authorization
    let context = AuthorizationContext {
        user_id: "test-user".to_string(),
        action: Action::Read,
        resource: "telemetry".to_string(),
        timestamp: chrono::Utc::now(),
        ip_address: "127.0.0.1".to_string(),
    };
    
    let auth_result = security_manager.authorize(&context).await;
    assert!(auth_result.is_ok());
    
    // Test encryption
    let test_data = b"Hello, World!";
    let encrypted = security_manager.encrypt_data(test_data).unwrap();
    let decrypted = security_manager.decrypt_data(&encrypted).unwrap();
    assert_eq!(test_data, decrypted.as_slice());
    
    // Test circuit breaker
    assert!(security_manager.can_execute_operation());
    
    // Test TLS
    assert!(!security_manager.is_tls_enabled()); // Not initialized yet
}

#[tokio::test]
async fn test_authentication_workflow() {
    let auth_config = AuthConfig {
        method: AuthMethod::Jwt,
        jwt_secret: Some("test-secret".to_string()),
        users: {
            let mut users = HashMap::new();
            users.insert("admin".to_string(), User {
                username: "admin".to_string(),
                password_hash: "hashed-password".to_string(),
                roles: vec!["admin".to_string()],
            });
            users
        },
    };
    
    let authorization_config = AuthorizationConfig {
        roles: HashMap::new(),
        user_roles: HashMap::new(),
        audit_logging: true,
    };
    
    let security_manager = SecurityManager::new(
        auth_config,
        authorization_config,
        None
    ).await.unwrap();
    
    // Test successful authentication
    let result = security_manager.authenticate("admin", "password").await;
    assert!(result.is_ok());
    
    // Test failed authentication
    let result = security_manager.authenticate("admin", "wrong-password").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_authorization_workflow() {
    let auth_config = AuthConfig {
        method: AuthMethod::None,
        jwt_secret: None,
        users: HashMap::new(),
    };
    
    let mut roles = HashMap::new();
    roles.insert("admin".to_string(), Role {
        name: "admin".to_string(),
        permissions: vec![
            Permission {
                action: Action::Read,
                resource: "telemetry".to_string(),
                conditions: PermissionConditions::default(),
            },
            Permission {
                action: Action::Write,
                resource: "telemetry".to_string(),
                conditions: PermissionConditions::default(),
            },
        ],
    });
    
    let mut user_roles = HashMap::new();
    user_roles.insert("test-user".to_string(), vec!["admin".to_string()]);
    
    let authorization_config = AuthorizationConfig {
        roles,
        user_roles,
        audit_logging: true,
    };
    
    let security_manager = SecurityManager::new(
        auth_config,
        authorization_config,
        None
    ).await.unwrap();
    
    // Test authorized access
    let context = AuthorizationContext {
        user_id: "test-user".to_string(),
        action: Action::Read,
        resource: "telemetry".to_string(),
        timestamp: chrono::Utc::now(),
        ip_address: "127.0.0.1".to_string(),
    };
    
    let result = security_manager.authorize(&context).await;
    assert!(result.is_ok());
    
    // Test unauthorized access
    let context = AuthorizationContext {
        user_id: "test-user".to_string(),
        action: Action::Delete,
        resource: "telemetry".to_string(),
        timestamp: chrono::Utc::now(),
        ip_address: "127.0.0.1".to_string(),
    };
    
    let result = security_manager.authorize(&context).await;
    assert!(result.is_err());
}

#[test]
fn test_encryption_workflow() {
    let auth_config = AuthConfig {
        method: AuthMethod::None,
        jwt_secret: None,
        users: HashMap::new(),
    };
    
    let authorization_config = AuthorizationConfig {
        roles: HashMap::new(),
        user_roles: HashMap::new(),
        audit_logging: false,
    };
    
    let security_manager = tokio::runtime::Runtime::new().unwrap().block_on(async {
        SecurityManager::new(auth_config, authorization_config, None).await.unwrap()
    });
    
    // Test data encryption and decryption
    let test_data = b"Sensitive telemetry data";
    let encrypted = security_manager.encrypt_data(test_data).unwrap();
    
    // Verify encrypted data is different
    assert_ne!(test_data, encrypted.as_slice());
    
    // Test decryption
    let decrypted = security_manager.decrypt_data(&encrypted).unwrap();
    assert_eq!(test_data, decrypted.as_slice());
    
    // Test with different data
    let test_data2 = b"Another sensitive message";
    let encrypted2 = security_manager.encrypt_data(test_data2).unwrap();
    let decrypted2 = security_manager.decrypt_data(&encrypted2).unwrap();
    assert_eq!(test_data2, decrypted2.as_slice());
}

#[tokio::test]
async fn test_circuit_breaker_workflow() {
    let auth_config = AuthConfig {
        method: AuthMethod::None,
        jwt_secret: None,
        users: HashMap::new(),
    };
    
    let authorization_config = AuthorizationConfig {
        roles: HashMap::new(),
        user_roles: HashMap::new(),
        audit_logging: false,
    };
    
    let security_manager = SecurityManager::new(auth_config, authorization_config, None).await.unwrap();
    
    // Test initial state
    assert!(security_manager.can_execute_operation());
    
    // Test failure threshold
    for _ in 0..5 {
        security_manager.record_operation_failure();
    }
    
    // Circuit should be open after 5 failures
    assert!(!security_manager.can_execute_operation());
    
    // Test recovery after timeout (using the default 60 second timeout, but with a simulated shorter wait)
    // For this test, we'll create a separate circuit breaker to test the timeout behavior
    let test_circuit_breaker = CircuitBreaker::new(5, std::time::Duration::from_millis(100));
    for _ in 0..5 {
        test_circuit_breaker.record_failure();
    }
    assert!(!test_circuit_breaker.can_execute());
    
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert!(test_circuit_breaker.can_execute());
    
    // Test successful recovery
    for _ in 0..3 {
        security_manager.record_operation_success();
    }
    
    // Circuit should be closed again
    assert!(security_manager.can_execute_operation());
}

#[test]
fn test_tls_workflow() {
    let auth_config = AuthConfig {
        method: AuthMethod::None,
        jwt_secret: None,
        users: HashMap::new(),
    };
    
    let authorization_config = AuthorizationConfig {
        roles: HashMap::new(),
        user_roles: HashMap::new(),
        audit_logging: false,
    };
    
    let tls_config = TlsConfig::default();
    
    let security_manager = tokio::runtime::Runtime::new().unwrap().block_on(async {
        SecurityManager::new(auth_config, authorization_config, Some(tls_config)).await.unwrap()
    });
    
    // Test TLS configuration validation
    let validation_result = security_manager.validate_tls_config();
    assert!(validation_result.is_err()); // Files don't exist
    
    // Test TLS initialization
    let init_result = security_manager.initialize_tls();
    assert!(init_result.is_err()); // Files don't exist
}

#[tokio::test]
async fn test_security_audit_logging() {
    let auth_config = AuthConfig {
        method: AuthMethod::None,
        jwt_secret: None,
        users: HashMap::new(),
    };
    
    let authorization_config = AuthorizationConfig {
        roles: HashMap::new(),
        user_roles: HashMap::new(),
        audit_logging: true,
    };
    
    let security_manager = SecurityManager::new(
        auth_config,
        authorization_config,
        None
    ).await.unwrap();
    
    // Test audit logging
    let context = AuthorizationContext {
        user_id: "test-user".to_string(),
        action: Action::Read,
        resource: "telemetry".to_string(),
        timestamp: chrono::Utc::now(),
        ip_address: "127.0.0.1".to_string(),
    };
    
    let audit_result = security_manager.authorize(&context).await;
    assert!(audit_result.is_ok());
    
    // Verify audit log was created
    let audit_logs = security_manager.get_audit_logs().await;
    assert!(!audit_logs.is_empty());
    
    // Check latest audit log
    let latest_log = audit_logs.last().unwrap();
    assert_eq!(latest_log.user_id, "test-user");
    assert_eq!(latest_log.action, Action::Read);
    assert_eq!(latest_log.resource, "telemetry");
}

#[tokio::test]
async fn test_security_error_handling() {
    let auth_config = AuthConfig {
        method: AuthMethod::Jwt,
        jwt_secret: Some("test-secret".to_string()),
        users: HashMap::new(),
    };
    
    let authorization_config = AuthorizationConfig {
        roles: HashMap::new(),
        user_roles: HashMap::new(),
        audit_logging: false,
    };
    
    let security_manager = SecurityManager::new(
        auth_config,
        authorization_config,
        None
    ).await.unwrap();
    
    // Test authentication with non-existent user
    let result = security_manager.authenticate("non-existent", "password").await;
    assert!(result.is_err());
    
    // Test authorization with non-existent user
    let context = AuthorizationContext {
        user_id: "non-existent".to_string(),
        action: Action::Read,
        resource: "telemetry".to_string(),
        timestamp: chrono::Utc::now(),
        ip_address: "127.0.0.1".to_string(),
    };
    
    let result = security_manager.authorize(&context).await;
    assert!(result.is_err());
    
    // Test encryption with invalid data
    let result = security_manager.decrypt_data(b"invalid-encrypted-data");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_security_performance() {
    let auth_config = AuthConfig {
        method: AuthMethod::None,
        jwt_secret: None,
        users: HashMap::new(),
    };
    
    let authorization_config = AuthorizationConfig {
        roles: HashMap::new(),
        user_roles: HashMap::new(),
        audit_logging: false,
    };
    
    let security_manager = SecurityManager::new(
        auth_config,
        authorization_config,
        None
    ).await.unwrap();
    
    // Test encryption performance
    let start = std::time::Instant::now();
    let test_data = b"Performance test data";
    
    for _ in 0..1000 {
        let encrypted = security_manager.encrypt_data(test_data).unwrap();
        let _decrypted = security_manager.decrypt_data(&encrypted).unwrap();
    }
    
    let duration = start.elapsed();
    assert!(duration.as_millis() < 1000); // Should complete within 1 second
    
    // Test authorization performance
    let start = std::time::Instant::now();
    let context = AuthorizationContext {
        user_id: "test-user".to_string(),
        action: Action::Read,
        resource: "telemetry".to_string(),
        timestamp: chrono::Utc::now(),
        ip_address: "127.0.0.1".to_string(),
    };
    
    for _ in 0..1000 {
        let _result = security_manager.authorize(&context).await;
    }
    
    let duration = start.elapsed();
    assert!(duration.as_millis() < 1000); // Should complete within 1 second
}
