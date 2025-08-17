#!/bin/bash

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CLUSTER_NAME="orasi-test"
NAMESPACE="orasi"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to test cluster connectivity
test_cluster() {
    print_status "Testing cluster connectivity..."
    
    if ! kubectl cluster-info >/dev/null 2>&1; then
        print_error "Cannot connect to cluster"
        return 1
    fi
    
    print_success "Cluster connectivity OK"
}

# Function to test nodes
test_nodes() {
    print_status "Testing cluster nodes..."
    
    local node_count
    node_count=$(kubectl get nodes --no-headers | wc -l)
    
    if [ "$node_count" -eq 3 ]; then
        print_success "All 3 nodes are ready"
    else
        print_error "Expected 3 nodes, found $node_count"
        return 1
    fi
    
    # Check if all nodes are ready
    local ready_nodes
    ready_nodes=$(kubectl get nodes --no-headers | grep -c "Ready")
    
    if [ "$ready_nodes" -eq 3 ]; then
        print_success "All nodes are in Ready state"
    else
        print_error "Not all nodes are ready"
        return 1
    fi
}

# Function to test namespace
test_namespace() {
    print_status "Testing namespace..."
    
    if kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
        print_success "Namespace '$NAMESPACE' exists"
    else
        print_error "Namespace '$NAMESPACE' does not exist"
        return 1
    fi
}

# Function to test pods
test_pods() {
    print_status "Testing pods..."
    
    local expected_pods=("bridge-api" "schema-registry" "web-ui")
    local failed_pods=()
    
    for pod in "${expected_pods[@]}"; do
        if kubectl get deployment "$pod" -n "$NAMESPACE" >/dev/null 2>&1; then
            local ready_replicas
            ready_replicas=$(kubectl get deployment "$pod" -n "$NAMESPACE" -o jsonpath='{.status.readyReplicas}')
            
            if [ "$ready_replicas" = "1" ]; then
                print_success "Deployment '$pod' is ready"
            else
                print_error "Deployment '$pod' is not ready (ready: $ready_replicas)"
                failed_pods+=("$pod")
            fi
        else
            print_error "Deployment '$pod' does not exist"
            failed_pods+=("$pod")
        fi
    done
    
    if [ ${#failed_pods[@]} -gt 0 ]; then
        return 1
    fi
}

# Function to test services
test_services() {
    print_status "Testing services..."
    
    local services=("bridge-api" "schema-registry" "web-ui")
    local ports=(30080 30081 30082)
    
    for i in "${!services[@]}"; do
        local service="${services[$i]}"
        local port="${ports[$i]}"
        
        if kubectl get service "$service" -n "$NAMESPACE" >/dev/null 2>&1; then
            print_success "Service '$service' exists"
            
            # Test if port is accessible
            if timeout 5 bash -c "</dev/tcp/localhost/$port" 2>/dev/null; then
                print_success "Service '$service' is accessible on port $port"
            else
                print_warning "Service '$service' is not accessible on port $port"
            fi
        else
            print_error "Service '$service' does not exist"
        fi
    done
}

# Function to test health endpoints
test_health_endpoints() {
    print_status "Testing health endpoints..."
    
    local endpoints=(
        "http://localhost:30080/health/live"
        "http://localhost:30081/health/live"
        "http://localhost:30082/health"
    )
    
    for endpoint in "${endpoints[@]}"; do
        if curl -s -f "$endpoint" >/dev/null 2>&1; then
            print_success "Health endpoint $endpoint is responding"
        else
            print_warning "Health endpoint $endpoint is not responding"
        fi
    done
}

# Function to test ingress
test_ingress() {
    print_status "Testing ingress..."
    
    if kubectl get ingress -n "$NAMESPACE" >/dev/null 2>&1; then
        print_success "Ingress exists"
    else
        print_warning "Ingress does not exist"
    fi
}

# Function to show cluster status
show_status() {
    print_status "Cluster status summary:"
    echo ""
    
    echo "Nodes:"
    kubectl get nodes
    echo ""
    
    echo "Pods in $NAMESPACE namespace:"
    kubectl get pods -n "$NAMESPACE"
    echo ""
    
    echo "Services in $NAMESPACE namespace:"
    kubectl get services -n "$NAMESPACE"
    echo ""
    
    echo "Deployments in $NAMESPACE namespace:"
    kubectl get deployments -n "$NAMESPACE"
    echo ""
}

# Main function
main() {
    print_status "Testing Orasi Kind cluster..."
    echo ""
    
    local tests_passed=0
    local tests_failed=0
    
    # Run tests
    if test_cluster; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_nodes; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_namespace; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_pods; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_services; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_health_endpoints; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    if test_ingress; then
        ((tests_passed++))
    else
        ((tests_failed++))
    fi
    
    echo ""
    print_status "Test results: $tests_passed passed, $tests_failed failed"
    
    if [ $tests_failed -eq 0 ]; then
        print_success "All tests passed! Cluster is ready for use."
        echo ""
        show_status
    else
        print_error "Some tests failed. Please check the cluster setup."
        echo ""
        show_status
        exit 1
    fi
}

# Run main function
main "$@"
