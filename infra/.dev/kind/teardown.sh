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

# Function to check if cluster exists
cluster_exists() {
    kind get clusters | grep -q "$CLUSTER_NAME"
}

# Function to delete namespace
delete_namespace() {
    print_status "Deleting namespace '$NAMESPACE'..."
    
    if kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
        kubectl delete namespace "$NAMESPACE"
        print_success "Namespace deleted"
    else
        print_warning "Namespace '$NAMESPACE' does not exist"
    fi
}

# Function to delete cluster
delete_cluster() {
    print_status "Deleting Kind cluster '$CLUSTER_NAME'..."
    
    if cluster_exists; then
        kind delete cluster --name "$CLUSTER_NAME"
        print_success "Cluster deleted"
    else
        print_warning "Cluster '$CLUSTER_NAME' does not exist"
    fi
}

# Function to clean up Docker images
cleanup_images() {
    print_status "Cleaning up Orasi Docker images..."
    
    # Remove Orasi images
    docker images | grep "orasi/" | awk '{print $3}' | xargs -r docker rmi -f
    
    print_success "Docker images cleaned up"
}

# Function to clean up Docker containers
cleanup_containers() {
    print_status "Cleaning up stopped containers..."
    
    # Remove stopped containers
    docker container prune -f
    
    print_success "Containers cleaned up"
}

# Function to clean up Docker volumes
cleanup_volumes() {
    print_status "Cleaning up unused volumes..."
    
    # Remove unused volumes
    docker volume prune -f
    
    print_success "Volumes cleaned up"
}

# Function to clean up Docker networks
cleanup_networks() {
    print_status "Cleaning up unused networks..."
    
    # Remove unused networks
    docker network prune -f
    
    print_success "Networks cleaned up"
}

# Function to show cleanup options
show_cleanup_options() {
    echo "Cleanup options:"
    echo "  1. Delete namespace only"
    echo "  2. Delete cluster only"
    echo "  3. Delete cluster and clean up Docker resources"
    echo "  4. Full cleanup (cluster + Docker + images)"
    echo "  5. Cancel"
    echo ""
}

# Function to perform full cleanup
full_cleanup() {
    print_status "Performing full cleanup..."
    
    delete_namespace
    delete_cluster
    cleanup_images
    cleanup_containers
    cleanup_volumes
    cleanup_networks
    
    print_success "Full cleanup completed!"
}

# Main function
main() {
    print_status "Orasi Kind cluster teardown..."
    
    if [ $# -eq 0 ]; then
        # Interactive mode
        show_cleanup_options
        read -p "Select cleanup option (1-5): " -n 1 -r
        echo
        
        case $REPLY in
            1)
                delete_namespace
                ;;
            2)
                delete_cluster
                ;;
            3)
                delete_namespace
                delete_cluster
                cleanup_containers
                cleanup_volumes
                cleanup_networks
                ;;
            4)
                full_cleanup
                ;;
            5)
                print_status "Cleanup cancelled"
                exit 0
                ;;
            *)
                print_error "Invalid option"
                exit 1
                ;;
        esac
    else
        # Command line mode
        case "$1" in
            "namespace")
                delete_namespace
                ;;
            "cluster")
                delete_cluster
                ;;
            "docker")
                cleanup_images
                cleanup_containers
                cleanup_volumes
                cleanup_networks
                ;;
            "full")
                full_cleanup
                ;;
            *)
                print_error "Invalid argument. Use: namespace, cluster, docker, or full"
                exit 1
                ;;
        esac
    fi
}

# Run main function
main "$@"
