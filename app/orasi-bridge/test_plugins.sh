#!/bin/bash

echo "Testing Orasi Bridge Plugin Discovery"
echo "====================================="

# Check if plugins directory exists
if [ ! -d "./plugins" ]; then
    echo "✗ Plugins directory not found"
    exit 1
fi

echo "✓ Plugins directory found"

# List discovered plugins
echo ""
echo "Discovered plugins:"
for plugin_dir in ./plugins/*/; do
    if [ -d "$plugin_dir" ]; then
        plugin_name=$(basename "$plugin_dir")
        echo "  - $plugin_name"
        
        # Check for plugin.json
        if [ -f "$plugin_dir/plugin.json" ]; then
            echo "    ✓ Has manifest file"
            
            # Extract plugin info from manifest
            name=$(jq -r '.name' "$plugin_dir/plugin.json" 2>/dev/null || echo "$plugin_name")
            version=$(jq -r '.version' "$plugin_dir/plugin.json" 2>/dev/null || echo "unknown")
            description=$(jq -r '.description' "$plugin_dir/plugin.json" 2>/dev/null || echo "No description")
            
            echo "    Name: $name"
            echo "    Version: $version"
            echo "    Description: $description"
            
            # Check capabilities
            capabilities=$(jq -r '.capabilities[]?' "$plugin_dir/plugin.json" 2>/dev/null | tr '\n' ', ' | sed 's/,$//')
            if [ -n "$capabilities" ]; then
                echo "    Capabilities: $capabilities"
            fi
        else
            echo "    ✗ No manifest file"
        fi
        
        # Check for executable
        if [ -f "$plugin_dir/plugin.py" ] && [ -x "$plugin_dir/plugin.py" ]; then
            echo "    ✓ Python plugin executable found"
        elif [ -f "$plugin_dir/plugin.js" ] && command -v node >/dev/null 2>&1; then
            echo "    ✓ JavaScript plugin executable found (Node.js available)"
        elif [ -f "$plugin_dir/processor.js" ] && command -v node >/dev/null 2>&1; then
            echo "    ✓ JavaScript processor executable found (Node.js available)"
        elif [ -f "$plugin_dir/plugin" ] && [ -x "$plugin_dir/plugin" ]; then
            echo "    ✓ Binary plugin executable found"
        else
            echo "    ✗ No executable plugin found"
        fi
        
        echo ""
    fi
done

# Test plugin health checks
echo "Testing plugin health checks:"
for plugin_dir in ./plugins/*/; do
    if [ -d "$plugin_dir" ]; then
        plugin_name=$(basename "$plugin_dir")
        
        # Test Python plugin
        if [ -f "$plugin_dir/plugin.py" ] && [ -x "$plugin_dir/plugin.py" ]; then
            echo "  Testing $plugin_name (Python)..."
            if python3 "$plugin_dir/plugin.py" health 2>/dev/null; then
                echo "    ✓ Health check passed"
            else
                echo "    ✗ Health check failed"
            fi
        fi
        
        # Test JavaScript plugin
        if [ -f "$plugin_dir/plugin.js" ] && command -v node >/dev/null 2>&1; then
            echo "  Testing $plugin_name (JavaScript)..."
            if node "$plugin_dir/plugin.js" health 2>/dev/null; then
                echo "    ✓ Health check passed"
            else
                echo "    ✗ Health check failed"
            fi
        elif [ -f "$plugin_dir/processor.js" ] && command -v node >/dev/null 2>&1; then
            echo "  Testing $plugin_name (JavaScript processor)..."
            if node "$plugin_dir/processor.js" health 2>/dev/null; then
                echo "    ✓ Health check passed"
            else
                echo "    ✗ Health check failed"
            fi
        fi
        
        # Test binary plugin
        if [ -f "$plugin_dir/plugin" ] && [ -x "$plugin_dir/plugin" ]; then
            echo "  Testing $plugin_name (Binary)..."
            if "$plugin_dir/plugin" health 2>/dev/null; then
                echo "    ✓ Health check passed"
            else
                echo "    ✗ Health check failed"
            fi
        fi
    fi
done

echo ""
echo "Plugin discovery test completed!"
