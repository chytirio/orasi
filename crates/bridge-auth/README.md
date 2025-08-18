# Bridge Auth

Authentication and authorization system for the Orasi OpenTelemetry Data Lake Bridge, providing secure access control and user management.

## Overview

Bridge Auth provides a comprehensive authentication and authorization system for the Orasi bridge, supporting:

- **Multiple Authentication Methods**: JWT, OAuth2, API Keys, and custom authentication
- **Role-Based Access Control (RBAC)**: Fine-grained permission management
- **User Management**: User registration, profile management, and session handling
- **Security Features**: Password hashing, rate limiting, and audit logging
- **Integration**: Seamless integration with the bridge ecosystem

## Key Features

### Authentication Methods
- **JWT Tokens**: JSON Web Token-based authentication
- **OAuth2**: OAuth2 provider integration (Google, GitHub, etc.)
- **API Keys**: Long-lived API key authentication
- **Session Management**: Secure session handling and storage
- **Multi-Factor Authentication**: TOTP and SMS-based MFA

### Authorization System
- **Role-Based Access Control**: Hierarchical role system
- **Permission Management**: Granular permission definitions
- **Resource Protection**: API endpoint and resource-level protection
- **Policy Enforcement**: Dynamic policy evaluation and enforcement

### Security Features
- **Password Security**: Argon2 password hashing
- **Rate Limiting**: Request rate limiting and throttling
- **Audit Logging**: Comprehensive security event logging
- **Token Management**: Secure token generation and validation
- **Session Security**: Secure session storage and management

## Quick Start

### Basic Usage

```rust
use bridge_auth::{
    auth::{AuthManager, AuthConfig},
    jwt::{JwtManager, JwtConfig},
    users::{UserManager, User},
    roles::{RoleManager, Role},
    permissions::{PermissionManager, Permission},
};

#[tokio::main]
async fn main() -> bridge_auth::Result<()> {
    // Initialize authentication manager
    let auth_config = AuthConfig::new()
        .with_jwt_secret("your-secret-key")
        .with_token_expiry(3600); // 1 hour
    
    let auth_manager = AuthManager::new(auth_config);
    
    // Initialize user manager
    let user_manager = UserManager::new();
    
    // Create a user
    let user = User::new(
        "john.doe@example.com".to_string(),
        "password123".to_string(),
        "John Doe".to_string(),
    );
    
    let user_id = user_manager.create_user(user).await?;
    
    // Create roles and permissions
    let role_manager = RoleManager::new();
    let permission_manager = PermissionManager::new();
    
    // Create admin permission
    let admin_permission = Permission::new(
        "admin".to_string(),
        "Full system access".to_string(),
        vec!["*".to_string()], // All resources
    );
    
    let permission_id = permission_manager.create_permission(admin_permission).await?;
    
    // Create admin role
    let admin_role = Role::new(
        "admin".to_string(),
        "System Administrator".to_string(),
        vec![permission_id],
    );
    
    let role_id = role_manager.create_role(admin_role).await?;
    
    // Assign role to user
    user_manager.assign_role(user_id, role_id).await?;
    
    // Authenticate user
    let credentials = bridge_auth::auth::Credentials {
        email: "john.doe@example.com".to_string(),
        password: "password123".to_string(),
    };
    
    let auth_result = auth_manager.authenticate(credentials).await?;
    
    if let Some(token) = auth_result.token {
        println!("Authentication successful! Token: {}", token);
        
        // Validate token
        let claims = auth_manager.validate_token(&token).await?;
        println!("User ID: {}, Roles: {:?}", claims.user_id, claims.roles);
    }
    
    Ok(())
}
```

### Middleware Integration

```rust
use axum::{
    routing::get,
    Router,
    middleware,
};
use bridge_auth::middleware::auth_middleware;

// Create protected routes
let app = Router::new()
    .route("/public", get(public_handler))
    .route("/protected", get(protected_handler))
    .layer(middleware::from_fn(auth_middleware));

async fn protected_handler(
    auth: bridge_auth::auth::AuthenticatedUser,
) -> String {
    format!("Hello, {}! You have access to this protected resource.", auth.user.email)
}
```

### Configuration Example

```toml
# auth.toml
[auth]
jwt_secret = "your-super-secret-jwt-key"
token_expiry_seconds = 3600
refresh_token_expiry_seconds = 2592000  # 30 days

[auth.oauth]
enabled = true
providers = ["google", "github"]

[auth.oauth.google]
client_id = "your-google-client-id"
client_secret = "your-google-client-secret"
redirect_uri = "http://localhost:3000/auth/callback/google"

[auth.oauth.github]
client_id = "your-github-client-id"
client_secret = "your-github-client-secret"
redirect_uri = "http://localhost:3000/auth/callback/github"

[auth.security]
password_min_length = 8
require_special_chars = true
max_login_attempts = 5
lockout_duration_seconds = 900  # 15 minutes

[auth.rate_limiting]
enabled = true
requests_per_minute = 100
burst_size = 20

[auth.session]
storage_type = "redis"
redis_url = "redis://localhost:6379"
session_expiry_seconds = 3600
```

## API Endpoints

### Authentication

```
POST /auth/login
POST /auth/logout
POST /auth/refresh
POST /auth/register
POST /auth/forgot-password
POST /auth/reset-password
```

### User Management

```
GET    /users
POST   /users
GET    /users/{id}
PUT    /users/{id}
DELETE /users/{id}
GET    /users/{id}/roles
POST   /users/{id}/roles
DELETE /users/{id}/roles/{role_id}
```

### Role Management

```
GET    /roles
POST   /roles
GET    /roles/{id}
PUT    /roles/{id}
DELETE /roles/{id}
GET    /roles/{id}/permissions
POST   /roles/{id}/permissions
DELETE /roles/{id}/permissions/{permission_id}
```

### Permission Management

```
GET    /permissions
POST   /permissions
GET    /permissions/{id}
PUT    /permissions/{id}
DELETE /permissions/{id}
```

## Architecture

Bridge Auth follows a modular architecture with clear separation of concerns:

```
┌─────────────────┐
│   Bridge Auth   │
├─────────────────┤
│  Authentication │
│  Authorization  │
│  User Mgmt      │
│  Role Mgmt      │
│  Permission Mgmt│
│  Security       │
│  Middleware     │
└─────────────────┘
```

### Core Components

1. **Authentication**: JWT, OAuth2, and API key authentication
2. **Authorization**: Role-based access control and permission management
3. **User Management**: User CRUD operations and profile management
4. **Role Management**: Role definition and assignment
5. **Permission Management**: Granular permission definitions
6. **Security**: Password hashing, rate limiting, and audit logging
7. **Middleware**: HTTP middleware for authentication and authorization

## Security Considerations

### Password Security
- Uses Argon2 for password hashing
- Configurable password complexity requirements
- Secure password reset flow

### Token Security
- JWT tokens with configurable expiration
- Secure token storage and transmission
- Token refresh mechanism

### Rate Limiting
- Configurable rate limiting per endpoint
- IP-based and user-based rate limiting
- Burst handling and graceful degradation

### Audit Logging
- Comprehensive security event logging
- User action tracking
- Failed authentication attempts logging

## Dependencies

### Core Dependencies
- **tokio**: Async runtime
- **serde**: Serialization/deserialization
- **tracing**: Structured logging
- **anyhow**: Error handling
- **thiserror**: Error type generation

### Authentication & Security
- **jsonwebtoken**: JWT token handling
- **argon2**: Password hashing
- **rand**: Random number generation
- **base64**: Base64 encoding/decoding
- **hmac**: HMAC for token signing
- **sha2**: SHA-2 hashing algorithms

### Web Framework
- **axum**: HTTP web framework
- **tower**: Middleware framework
- **tower-http**: HTTP middleware
- **hyper**: HTTP client/server

### Utilities
- **async-trait**: Async trait support
- **futures**: Future utilities
- **dashmap**: Concurrent hash map

## Development

### Building

```bash
# Build the crate
cargo build

# Build with optimizations
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo test
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_jwt_authentication

# Run integration tests
cargo test --test integration
```

### Documentation

```bash
# Generate documentation
cargo doc

# Open documentation in browser
cargo doc --open
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
