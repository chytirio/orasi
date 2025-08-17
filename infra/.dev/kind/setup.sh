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
CONFIG_FILE="kind-config.yaml"
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

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
    local missing_commands=()
    
    if ! command_exists kind; then
        missing_commands+=("kind")
    fi
    
    if ! command_exists kubectl; then
        missing_commands+=("kubectl")
    fi
    
    if ! command_exists docker; then
        missing_commands+=("docker")
    fi
    
    if [ ${#missing_commands[@]} -ne 0 ]; then
        print_error "Missing required commands: ${missing_commands[*]}"
        print_status "Please install the missing commands:"
        echo "  kind: https://kind.sigs.k8s.io/docs/user/quick-start/#installation"
        echo "  kubectl: https://kubernetes.io/docs/tasks/tools/install-kubectl/"
        echo "  docker: https://docs.docker.com/get-docker/"
        exit 1
    fi
    
    print_success "All prerequisites are installed"
}

# Function to create cluster
create_cluster() {
    print_status "Creating Kind cluster '$CLUSTER_NAME'..."
    
    if kind get clusters | grep -q "$CLUSTER_NAME"; then
        print_warning "Cluster '$CLUSTER_NAME' already exists"
        read -p "Do you want to delete and recreate it? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_status "Deleting existing cluster..."
            kind delete cluster --name "$CLUSTER_NAME"
        else
            print_status "Using existing cluster"
            return 0
        fi
    fi
    
    kind create cluster --name "$CLUSTER_NAME" --config "$CONFIG_FILE"
    
    if [ $? -eq 0 ]; then
        print_success "Cluster created successfully"
    else
        print_error "Failed to create cluster"
        exit 1
    fi
}

# Function to configure kubectl context
configure_kubectl() {
    print_status "Configuring kubectl context..."
    
    kind export kubeconfig --name "$CLUSTER_NAME"
    
    # Wait for cluster to be ready
    print_status "Waiting for cluster to be ready..."
    kubectl wait --for=condition=Ready nodes --all --timeout=300s
    
    print_success "Kubectl context configured"
}

# Function to install addons
install_addons() {
    print_status "Installing cluster addons..."
    
    # Install Calico CNI
    print_status "Installing Calico CNI..."
    kubectl apply -f https://raw.githubusercontent.com/projectcalico/calico/v3.26.1/manifests/calico.yaml
    
    # Wait for Calico to be ready
    kubectl wait --for=condition=Ready pods --all -n kube-system --timeout=300s
    
    # Install metrics server
    print_status "Installing metrics server..."
    kubectl apply -f https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml
    
    # Patch metrics server to work with Kind
    kubectl patch deployment metrics-server -n kube-system --type='json' -p='[{"op": "add", "path": "/spec/template/spec/containers/0/args/-", "value": "--kubelet-insecure-tls"}]'
    
    # Install NGINX Ingress Controller
    print_status "Installing NGINX Ingress Controller..."
    kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml
    
    # Wait for ingress controller to be ready
    kubectl wait --namespace ingress-nginx \
        --for=condition=ready pod \
        --selector=app.kubernetes.io/component=controller \
        --timeout=300s
    
    print_success "Addons installed successfully"
}

# Function to create namespace
create_namespace() {
    print_status "Creating namespace '$NAMESPACE'..."
    
    kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -
    
    print_success "Namespace created"
}

# Function to build and load Orasi images
build_and_load_images() {
    print_status "Building and loading Orasi images..."
    
    # Get the project root directory
    local project_root
    project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
    
    # Build images using Docker Bake
    print_status "Building Docker images..."
    cd "$project_root"
    
    # Build all images
    if command_exists just; then
        just docker-bake-all
    else
        # Fallback to direct docker build
        docker build -f infra/docker/Dockerfile -t orasi/bridge-api:latest .
        docker build -f infra/docker/Dockerfile -t orasi/schema-registry:latest .
        docker build -f infra/docker/Dockerfile -t orasi/web:latest .
    fi
    
    # Load images into Kind cluster
    print_status "Loading images into Kind cluster..."
    kind load docker-image orasi/bridge-api:latest --name "$CLUSTER_NAME"
    kind load docker-image orasi/schema-registry:latest --name "$CLUSTER_NAME"
    kind load docker-image orasi/web:latest --name "$CLUSTER_NAME"
    
    print_success "Images loaded successfully"
}

# Function to deploy Orasi services
deploy_orasi_services() {
    print_status "Deploying Orasi services..."
    
    # Apply Kubernetes manifests
    kubectl apply -f k8s/ -n "$NAMESPACE"
    
    # Wait for services to be ready
    print_status "Waiting for services to be ready..."
    kubectl wait --for=condition=Available deployment --all -n "$NAMESPACE" --timeout=300s
    
    print_success "Services deployed successfully"
}

# Function to show cluster information
show_cluster_info() {
    print_status "Cluster information:"
    echo "  Cluster name: $CLUSTER_NAME"
    echo "  Namespace: $NAMESPACE"
    echo ""
    echo "  Services:"
    echo "    Bridge API: http://localhost:30080"
    echo "    Schema Registry: http://localhost:30081"
    echo "    Web UI: http://localhost:30082"
    echo ""
    echo "  Useful commands:"
    echo "    kubectl get pods -n $NAMESPACE"
    echo "    kubectl logs -f deployment/bridge-api -n $NAMESPACE"
    echo "    kind delete cluster --name $CLUSTER_NAME"
}

# Main function
main() {
    print_status "Setting up Kind cluster for Orasi testing..."
    
    check_prerequisites
    create_cluster
    configure_kubectl
    install_addons
    create_namespace
    build_and_load_images
    deploy_orasi_services
    show_cluster_info
    
    print_success "Kind cluster setup completed!"
}

# Run main function
main "$@"
