#!/usr/bin/env python3
"""
Example plugin for Orasi Bridge
"""

import json
import sys
import time
from datetime import datetime

def main():
    """Main plugin entry point"""
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No command specified"}))
        sys.exit(1)
    
    command = sys.argv[1]
    
    if command == "health":
        # Health check
        print(json.dumps({
            "status": "healthy",
            "timestamp": datetime.utcnow().isoformat(),
            "version": "1.0.0"
        }))
    elif command == "query":
        # Query execution
        if len(sys.argv) < 3:
            print(json.dumps({"error": "No query data provided"}))
            sys.exit(1)
        
        query_data = sys.argv[2]
        try:
            data = json.loads(query_data)
            result = process_query(data)
            print(json.dumps(result))
        except json.JSONDecodeError:
            print(json.dumps({"error": "Invalid JSON query data"}))
            sys.exit(1)
    else:
        print(json.dumps({"error": f"Unknown command: {command}"}))
        sys.exit(1)

def process_query(data):
    """Process a query request"""
    return {
        "plugin": "example-plugin",
        "type": "query",
        "data": data,
        "timestamp": datetime.utcnow().isoformat(),
        "status": "success",
        "result_count": 1,
        "results": {
            "message": "Query processed by example plugin",
            "processed_data": data
        }
    }

if __name__ == "__main__":
    main()
