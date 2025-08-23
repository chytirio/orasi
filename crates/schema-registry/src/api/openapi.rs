//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! OpenAPI documentation for the Schema Registry API
//!
//! This module provides OpenAPI/Swagger documentation for the schema registry API.

use serde_json::json;

/// Generate OpenAPI documentation
pub fn generate_openapi_docs() -> serde_json::Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Orasi Schema Registry API",
            "description": "API for managing and validating OpenTelemetry telemetry schemas",
            "version": "1.0.0",
            "contact": {
                "name": "Orasi Team",
                "url": "https://github.com/chytirio/orasi"
            },
            "license": {
                "name": "Apache 2.0",
                "url": "https://www.apache.org/licenses/LICENSE-2.0"
            }
        },
        "servers": [
            {
                "url": "http://localhost:8080",
                "description": "Development server"
            }
        ],
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "description": "Check the health status of the schema registry",
                    "tags": ["Health"],
                    "responses": {
                        "200": {
                            "description": "Service is healthy",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/HealthResponse"
                                    }
                                }
                            }
                        },
                        "503": {
                            "description": "Service is unhealthy"
                        }
                    }
                }
            },
            "/schemas": {
                "post": {
                    "summary": "Register schema",
                    "description": "Register a new schema in the registry",
                    "tags": ["Schemas"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/RegisterSchemaRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Schema registered successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/RegisterSchemaResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid request"
                        },
                        "409": {
                            "description": "Schema already exists"
                        }
                    }
                },
                "get": {
                    "summary": "List schemas",
                    "description": "List all schemas in the registry",
                    "tags": ["Schemas"],
                    "parameters": [
                        {
                            "name": "name",
                            "in": "query",
                            "description": "Filter by schema name",
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "version",
                            "in": "query",
                            "description": "Filter by schema version",
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "format",
                            "in": "query",
                            "description": "Filter by schema format",
                            "schema": {
                                "type": "string",
                                "enum": ["json", "yaml", "avro", "protobuf", "openapi"]
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "List of schemas",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ListSchemasResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/schemas/{fingerprint}": {
                "get": {
                    "summary": "Get schema",
                    "description": "Retrieve a schema by its fingerprint",
                    "tags": ["Schemas"],
                    "parameters": [
                        {
                            "name": "fingerprint",
                            "in": "path",
                            "required": true,
                            "description": "Schema fingerprint",
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Schema found",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/GetSchemaResponse"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Schema not found"
                        }
                    }
                },
                "delete": {
                    "summary": "Delete schema",
                    "description": "Delete a schema from the registry",
                    "tags": ["Schemas"],
                    "parameters": [
                        {
                            "name": "fingerprint",
                            "in": "path",
                            "required": true,
                            "description": "Schema fingerprint",
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Schema deleted successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/DeleteSchemaResponse"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Schema not found"
                        }
                    }
                }
            },
            "/schemas/validate": {
                "post": {
                    "summary": "Validate schema",
                    "description": "Validate a schema without registering it",
                    "tags": ["Validation"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ValidateSchemaRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Validation result",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ValidateSchemaResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid schema"
                        }
                    }
                }
            },
            "/schemas/{fingerprint}/validate": {
                "post": {
                    "summary": "Validate data against schema",
                    "description": "Validate telemetry data against a registered schema",
                    "tags": ["Validation"],
                    "parameters": [
                        {
                            "name": "fingerprint",
                            "in": "path",
                            "required": true,
                            "description": "Schema fingerprint",
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ValidateDataRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Validation result",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ValidateDataResponse"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Schema not found"
                        }
                    }
                }
            },
            "/schemas/evolve": {
                "post": {
                    "summary": "Evolve schema",
                    "description": "Evolve a schema with backward/forward compatibility checks",
                    "tags": ["Schemas"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/EvolveSchemaRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Schema evolved successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/EvolveSchemaResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Invalid schema or compatibility issues"
                        }
                    }
                }
            },
            "/metrics": {
                "get": {
                    "summary": "Get metrics",
                    "description": "Get Prometheus metrics",
                    "tags": ["Monitoring"],
                    "responses": {
                        "200": {
                            "description": "Metrics in Prometheus format",
                            "content": {
                                "text/plain": {
                                    "schema": {
                                        "type": "string"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/stats": {
                "get": {
                    "summary": "Get statistics",
                    "description": "Get registry statistics",
                    "tags": ["Monitoring"],
                    "responses": {
                        "200": {
                            "description": "Registry statistics",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/StatsResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "HealthResponse": {
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["healthy", "unhealthy"]
                        },
                        "timestamp": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "initialized": {
                            "type": "boolean"
                        },
                        "last_health_check": {
                            "type": "string",
                            "format": "date-time"
                        }
                    },
                    "required": ["status", "timestamp"]
                },
                "RegisterSchemaRequest": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Schema name"
                        },
                        "version": {
                            "type": "string",
                            "description": "Schema version"
                        },
                        "schema_type": {
                            "type": "string",
                            "description": "Schema type"
                        },
                        "content": {
                            "type": "string",
                            "description": "Schema content"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["json", "yaml", "avro", "protobuf", "openapi"]
                        }
                    },
                    "required": ["name", "version", "content", "format"]
                },
                "RegisterSchemaResponse": {
                    "type": "object",
                    "properties": {
                        "fingerprint": {
                            "type": "string",
                            "description": "Schema fingerprint"
                        },
                        "version": {
                            "type": "string",
                            "description": "Schema version"
                        },
                        "message": {
                            "type": "string",
                            "description": "Success message"
                        }
                    },
                    "required": ["fingerprint", "version", "message"]
                },
                "ListSchemasResponse": {
                    "type": "object",
                    "properties": {
                        "schemas": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/SchemaMetadata"
                            }
                        },
                        "total_count": {
                            "type": "integer"
                        }
                    },
                    "required": ["schemas", "total_count"]
                },
                "GetSchemaResponse": {
                    "type": "object",
                    "properties": {
                        "schema": {
                            "$ref": "#/components/schemas/Schema"
                        }
                    },
                    "required": ["schema"]
                },
                "DeleteSchemaResponse": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Success message"
                        }
                    },
                    "required": ["message"]
                },
                "ValidateSchemaRequest": {
                    "type": "object",
                    "properties": {
                        "schema": {
                            "$ref": "#/components/schemas/Schema"
                        }
                    },
                    "required": ["schema"]
                },
                "ValidateSchemaResponse": {
                    "type": "object",
                    "properties": {
                        "valid": {
                            "type": "boolean"
                        },
                        "status": {
                            "type": "string"
                        },
                        "error_count": {
                            "type": "integer"
                        },
                        "warning_count": {
                            "type": "integer"
                        },
                        "errors": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "warnings": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        }
                    },
                    "required": ["valid", "status", "error_count", "warning_count"]
                },
                "ValidateDataRequest": {
                    "type": "object",
                    "properties": {
                        "data": {
                            "type": "object",
                            "description": "Telemetry data to validate"
                        }
                    },
                    "required": ["data"]
                },
                "ValidateDataResponse": {
                    "type": "object",
                    "properties": {
                        "valid": {
                            "type": "boolean"
                        },
                        "status": {
                            "type": "string"
                        },
                        "error_count": {
                            "type": "integer"
                        },
                        "warning_count": {
                            "type": "integer"
                        },
                        "errors": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "warnings": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        }
                    },
                    "required": ["valid", "status", "error_count", "warning_count"]
                },
                "EvolveSchemaRequest": {
                    "type": "object",
                    "properties": {
                        "schema": {
                            "$ref": "#/components/schemas/Schema"
                        }
                    },
                    "required": ["schema"]
                },
                "EvolveSchemaResponse": {
                    "type": "object",
                    "properties": {
                        "fingerprint": {
                            "type": "string",
                            "description": "Schema fingerprint"
                        },
                        "version": {
                            "type": "string",
                            "description": "Schema version"
                        },
                        "message": {
                            "type": "string",
                            "description": "Success message"
                        }
                    },
                    "required": ["fingerprint", "version", "message"]
                },
                "StatsResponse": {
                    "type": "object",
                    "properties": {
                        "stats": {
                            "type": "object",
                            "description": "Registry statistics"
                        }
                    },
                    "required": ["stats"]
                },
                "Schema": {
                    "type": "object",
                    "properties": {
                        "fingerprint": {
                            "type": "string"
                        },
                        "name": {
                            "type": "string"
                        },
                        "version": {
                            "type": "string"
                        },
                        "schema_type": {
                            "type": "string"
                        },
                        "content": {
                            "type": "string"
                        },
                        "format": {
                            "type": "string"
                        },
                        "metadata": {
                            "$ref": "#/components/schemas/SchemaMetadata"
                        }
                    },
                    "required": ["fingerprint", "name", "version", "content", "format"]
                },
                "SchemaMetadata": {
                    "type": "object",
                    "properties": {
                        "fingerprint": {
                            "type": "string"
                        },
                        "name": {
                            "type": "string"
                        },
                        "version": {
                            "type": "string"
                        },
                        "schema_type": {
                            "type": "string"
                        },
                        "format": {
                            "type": "string"
                        },
                        "created_at": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "updated_at": {
                            "type": "string",
                            "format": "date-time"
                        }
                    },
                    "required": ["fingerprint", "name", "version", "schema_type", "format"]
                }
            },
            "securitySchemes": {
                "ApiKeyAuth": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-API-Key"
                }
            }
        },
        "security": [
            {
                "ApiKeyAuth": []
            }
        ],
        "tags": [
            {
                "name": "Health",
                "description": "Health check endpoints"
            },
            {
                "name": "Schemas",
                "description": "Schema management endpoints"
            },
            {
                "name": "Validation",
                "description": "Schema validation endpoints"
            },
            {
                "name": "Monitoring",
                "description": "Monitoring and metrics endpoints"
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let docs = generate_openapi_docs();

        // Verify basic structure
        assert_eq!(docs["openapi"], "3.0.3");
        assert_eq!(docs["info"]["title"], "Orasi Schema Registry API");
        assert_eq!(docs["info"]["version"], "1.0.0");

        // Verify paths exist
        assert!(docs["paths"]["/health"].is_object());
        assert!(docs["paths"]["/schemas"].is_object());
        assert!(docs["paths"]["/metrics"].is_object());

        // Verify components exist
        assert!(docs["components"]["schemas"].is_object());
        assert!(docs["components"]["securitySchemes"].is_object());
    }
}
