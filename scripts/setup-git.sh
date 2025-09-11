#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Check if git is installed
check_git() {
    if ! command -v git &> /dev/null; then
        print_error "Git is not installed. Please install git first."
        exit 1
    fi
    print_status "Git is available"
}

# Initialize git repository
init_git() {
    if [ -d ".git" ]; then
        print_warning "Git repository already exists"
        return
    fi
    
    print_info "Initializing Git repository..."
    git init
    print_status "Git repository initialized"
}

# Add remote repository
setup_remote() {
    local repo_url=$1
    
    if [ -z "$repo_url" ]; then
        print_warning "No repository URL provided. You can add it later with:"
        print_info "git remote add origin <your-repo-url>"
        return
    fi
    
    print_info "Adding remote repository: $repo_url"
    git remote add origin "$repo_url"
    print_status "Remote repository added"
}

# Create initial commit
create_initial_commit() {
    print_info "Adding files to Git..."
    git add .
    
    print_info "Creating initial commit..."
    git commit -m "Initial commit: cert-agent mTLS certificate management service

- gRPC API for certificate management
- Redis integration for certificate tracking
- Automatic certificate renewal
- Debian package support with debconf
- Docker and Docker Compose support
- GitHub Actions CI/CD workflows
- Comprehensive documentation"
    
    print_status "Initial commit created"
}

# Setup branches
setup_branches() {
    print_info "Setting up branches..."
    
    # Create develop branch
    git checkout -b develop
    print_status "Created develop branch"
    
    # Return to main
    git checkout main
    print_status "Branches setup completed"
}

# Setup GitHub repository
setup_github() {
    local repo_name=${1:-"cert-agent"}
    local org_name=${2:-"$(git config user.name)"}
    
    print_info "Setting up GitHub repository..."
    print_info "Repository: $org_name/$repo_name"
    
    # Create repository on GitHub (requires GitHub CLI)
    if command -v gh &> /dev/null; then
        print_info "Creating GitHub repository using GitHub CLI..."
        gh repo create "$org_name/$repo_name" \
            --public \
            --description "Modern mTLS certificate management service with gRPC API and Redis integration" \
            --homepage "https://github.com/$org_name/$repo_name" \
            --add-readme \
            --clone=false
        
        print_status "GitHub repository created"
    else
        print_warning "GitHub CLI not found. Please create repository manually:"
        print_info "1. Go to https://github.com/new"
        print_info "2. Create repository: $repo_name"
        print_info "3. Add remote: git remote add origin git@github.com:$org_name/$repo_name.git"
    fi
}

# Main setup function
main() {
    local repo_url=$1
    local repo_name=$2
    local org_name=$3
    
    echo -e "${BLUE}🚀 Setting up Git repository for cert-agent${NC}"
    
    check_git
    init_git
    setup_remote "$repo_url"
    create_initial_commit
    setup_branches
    
    if [ -n "$repo_name" ]; then
        setup_github "$repo_name" "$org_name"
    fi
    
    echo -e "${GREEN}🎉 Git repository setup completed!${NC}"
    echo -e "${BLUE}Next steps:${NC}"
    echo -e "1. Push to remote: ${YELLOW}git push -u origin main${NC}"
    echo -e "2. Push develop branch: ${YELLOW}git push -u origin develop${NC}"
    echo -e "3. Create first release: ${YELLOW}git tag v0.1.0 && git push origin v0.1.0${NC}"
    echo -e "4. Check GitHub Actions: ${YELLOW}Visit your repository on GitHub${NC}"
}

# Show usage
usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --repo-url URL      Git repository URL"
    echo "  --repo-name NAME    GitHub repository name"
    echo "  --org-name NAME     GitHub organization/user name"
    echo "  --help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --repo-url git@github.com:myorg/cert-agent.git"
    echo "  $0 --repo-name cert-agent --org-name myorg"
}

# Parse command line arguments
REPO_URL=""
REPO_NAME=""
ORG_NAME=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --repo-url)
            REPO_URL="$2"
            shift 2
            ;;
        --repo-name)
            REPO_NAME="$2"
            shift 2
            ;;
        --org-name)
            ORG_NAME="$2"
            shift 2
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Run main function
main "$REPO_URL" "$REPO_NAME" "$ORG_NAME"
