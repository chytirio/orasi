# Configuration Management

The Bridge API provides comprehensive configuration management capabilities that allow you to dynamically update, validate, and manage configuration settings at runtime.

## Overview

The configuration management system supports:

- **Dynamic Configuration Updates**: Update configuration sections without restarting the service
- **Validation**: Comprehensive validation of configuration changes before application
- **Hot Reloading**: Apply configuration changes immediately when requested
- **Backup & Restore**: Automatic backup creation before applying changes
- **Change History**: Detailed logging of all configuration changes
- **Section-based Updates**: Update specific configuration sections independently

## Configuration Sections

The Bridge API supports the following configuration sections:

### HTTP Configuration (`http`)
- Server address and port
- Request timeouts
- Compression settings
- Keep-alive configuration

### gRPC Configuration (`grpc`)
- Server address and port
- Message size limits
- Reflection settings
- Concurrent stream limits

### Authentication Configuration (`auth`)
- JWT secret management
- Authentication enable/disable
- Token validation settings

### Rate Limiting Configuration (`rate_limit`)
- Request rate limits
- Burst limits
- Rate limiting enable/disable

### Logging Configuration (`logging`)
- Log levels (trace, debug, info, warn, error)
- Log format settings
- Output destinations

### Metrics Configuration (`metrics`)
- Metrics collection enable/disable
- Metrics endpoint configuration
- Export settings

### Security Configuration (`security`)
- TLS settings
- Certificate management
- Security headers

### CORS Configuration (`cors`)
- Allowed origins
- CORS enable/disable
- Cross-origin settings

## API Usage

### Apply Configuration Changes

```bash
POST /api/v1/config
Content-Type: application/json

{
  "section": "http",
  "data": {
    "port": 9090,
    "address": "0.0.0.0",
    "request_timeout": 60
  },
  "options": {
    "validate": true,
    "hot_reload": true,
    "backup": true
  }
}
```

### Get Current Configuration

```bash
GET /api/v1/config
```

### Update Configuration

```bash
PUT /api/v1/config
Content-Type: application/json

{
  "http": {
    "port": 9090,
    "address": "0.0.0.0"
  },
  "grpc": {
    "port": 9091
  }
}
```

### Validate Configuration

```bash
POST /api/v1/config/validate
Content-Type: application/json

{
  "http": {
    "port": 9090,
    "address": "0.0.0.0"
  }
}
```

## Configuration Options

### Validation Options

- `validate`: Enable/disable configuration validation (default: true)
- `hot_reload`: Enable/disable hot reloading (default: false)
- `backup`: Enable/disable automatic backup creation (default: false)

## Validation Rules

### HTTP Configuration
- Port must be between 1 and 65535
- Address cannot be empty
- Request timeout must be greater than 0
- Max request size must be greater than 0

### gRPC Configuration
- Port must be between 1 and 65535
- Address cannot be empty
- Max message size must be greater than 0
- Max concurrent streams must be greater than 0

### Authentication Configuration
- JWT secret must be at least 32 characters long
- JWT secret cannot be empty
- Enabled flag must be boolean

### Rate Limiting Configuration
- Requests per minute must be greater than 0
- Enabled flag must be boolean

### Logging Configuration
- Log level must be one of: trace, debug, info, warn, error

### Metrics Configuration
- Port must be between 1 and 65535
- Enabled flag must be boolean

### Security Configuration
- TLS enabled flag must be boolean
- Certificate path cannot be empty if TLS is enabled

### CORS Configuration
- Enabled flag must be boolean
- Allowed origins must be an array of strings

## File Structure

The configuration management system creates the following directory structure:

```
config/
├── backups/
│   ├── config_backup_20250823_022825.json
│   └── ...
└── history/
    ├── config_history_20250823.json
    └── ...
```

### Backup Files

Backup files contain the complete configuration state before changes are applied. They are named with timestamps for easy identification and rollback.

### History Files

History files contain detailed logs of all configuration changes, including:
- Timestamp of the change
- Configuration section modified
- List of changes applied
- User who made the change

## Error Handling

The configuration management system provides comprehensive error handling:

### Validation Errors
- Invalid configuration values
- Missing required fields
- Type mismatches
- Range violations

### Application Errors
- File system errors
- Permission issues
- Resource conflicts

### Response Format

```json
{
  "data": {
    "status": "applied|validation_failed|partial_failure",
    "config_hash": "abc123...",
    "changes": [
      "Configuration validated successfully",
      "Configuration backup created",
      "Configuration section 'http' applied successfully"
    ],
    "validation_errors": [
      "Port must be between 1 and 65535"
    ]
  },
  "metadata": {
    "request_id": "uuid",
    "timestamp": "2025-08-23T02:28:25.882473+00:00",
    "version": "0.1.0",
    "processing_time_ms": 150
  }
}
```

## Environment Variables

The following environment variables control the configuration management behavior:

- `BRIDGE_CONFIG_BACKUP_DIR`: Directory for configuration backups (default: `./config/backups`)
- `BRIDGE_CONFIG_HISTORY_DIR`: Directory for configuration history (default: `./config/history`)

## Best Practices

### 1. Always Validate Configuration
```json
{
  "options": {
    "validate": true
  }
}
```

### 2. Create Backups for Critical Changes
```json
{
  "options": {
    "backup": true
  }
}
```

### 3. Use Hot Reload for Non-Critical Changes
```json
{
  "options": {
    "hot_reload": true
  }
}
```

### 4. Test Configuration Changes
Always test configuration changes in a development environment before applying to production.

### 5. Monitor Configuration History
Regularly review the configuration history to track changes and identify potential issues.

## Example Usage

### Update HTTP Port
```bash
curl -X POST http://localhost:8080/api/v1/config \
  -H "Content-Type: application/json" \
  -d '{
    "section": "http",
    "data": {
      "port": 9090
    },
    "options": {
      "validate": true,
      "backup": true
    }
  }'
```

### Update Multiple Sections
```bash
curl -X PUT http://localhost:8080/api/v1/config \
  -H "Content-Type: application/json" \
  -d '{
    "http": {
      "port": 9090,
      "request_timeout": 60
    },
    "logging": {
      "level": "info"
    }
  }'
```

### Validate Configuration
```bash
curl -X POST http://localhost:8080/api/v1/config/validate \
  -H "Content-Type: application/json" \
  -d '{
    "http": {
      "port": 0
    }
  }'
```

## Troubleshooting

### Common Issues

1. **Validation Failed**: Check the validation rules for the specific configuration section
2. **Permission Denied**: Ensure the application has write permissions to the backup and history directories
3. **Hot Reload Failed**: Some configuration changes may require a service restart
4. **Backup Creation Failed**: Check available disk space and directory permissions

### Debug Information

Enable debug logging to get detailed information about configuration changes:

```bash
export RUST_LOG=bridge_api::handlers::config=debug
```

## Security Considerations

1. **JWT Secrets**: Always use strong, randomly generated secrets
2. **File Permissions**: Restrict access to backup and history directories
3. **Network Security**: Use HTTPS for configuration API endpoints in production
4. **Audit Logging**: Monitor configuration changes for security compliance

## Integration with Monitoring

The configuration management system integrates with the monitoring system to provide:

- Configuration change metrics
- Validation error rates
- Hot reload success rates
- Backup operation status

Monitor these metrics to ensure the configuration management system is operating correctly.
