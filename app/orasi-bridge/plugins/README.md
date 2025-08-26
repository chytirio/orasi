# Orasi Bridge Plugin System

This directory contains plugins for the Orasi Bridge system. The plugin system allows for dynamic discovery and execution of custom functionality.

## Plugin Structure

Each plugin should be organized in its own directory with the following structure:

```
plugin-name/
├── plugin.json          # Plugin manifest (required)
├── plugin.py           # Python plugin executable (optional)
├── plugin.js           # JavaScript plugin executable (optional)
├── plugin              # Binary plugin executable (optional)
├── config.json         # Plugin configuration (optional)
└── .health             # Health status file (auto-generated)
```

## Plugin Manifest (plugin.json)

The plugin manifest file defines the plugin's metadata and capabilities:

```json
{
  "name": "plugin-name",
  "version": "1.0.0",
  "description": "Plugin description",
  "capabilities": [
    "query",
    "analytics",
    "custom_processing"
  ],
  "author": "Plugin Author",
  "license": "Apache-2.0",
  "main": "plugin.py",
  "dependencies": [],
  "config": {
    "enabled": true,
    "timeout": 30,
    "max_results": 1000
  }
}
```

## Plugin Executables

Plugins can be implemented in any language that can be executed from the command line. The plugin system supports:

- **Python**: `plugin.py` (must be executable)
- **JavaScript**: `plugin.js` (must be executable)
- **Binary**: `plugin` (compiled executable)
- **Windows**: `plugin.exe`

## Plugin Interface

Plugins should support the following command-line interface:

### Health Check
```bash
./plugin health
```
Should return JSON response:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "1.0.0"
}
```

### Query Execution
```bash
./plugin query '{"query": "data", "filters": {}}'
```
Should return JSON response with query results.

## Built-in Plugins

The system includes several built-in plugins:

1. **analytics**: Data analysis and visualization
2. **query-engine**: SQL and data querying
3. **streaming-processor**: Real-time data processing

## Plugin Discovery

The plugin system automatically discovers plugins by:

1. Scanning the plugins directory (configurable via `ORASI_PLUGINS_DIR` environment variable)
2. Reading plugin manifests
3. Checking plugin health status
4. Registering available capabilities

## Plugin Status

Plugins can have the following statuses:

- **Active**: Plugin is healthy and ready to use
- **Inactive**: Plugin is not properly configured
- **Loading**: Plugin is being initialized
- **Error**: Plugin has encountered an error

## Configuration

Set the plugins directory using the environment variable:
```bash
export ORASI_PLUGINS_DIR=/path/to/plugins
```

Default location: `./plugins` (relative to the bridge executable)

## Example Plugins

### example-plugin
A simple Python plugin demonstrating basic query functionality.

### data-processor
A JavaScript plugin showing advanced data processing capabilities.

## API Endpoints

- `GET /plugin/capabilities` - List all available plugins and capabilities
- `POST /plugin/query` - Execute a plugin query
- `GET /plugin/stream` - Stream plugin updates
- `POST /plugin/analytics` - Execute analytics plugin operations
