# ===================================================================
# ZK Private Lending - Frontend Dockerfile
# ===================================================================
#
# Multi-stage build for optimized production image
#
# Build: docker build -t zk-lending-frontend ./frontend
# Run:   docker run -p 5173:80 zk-lending-frontend
#
# ===================================================================

# Stage 1: Build
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm ci

# Copy source code
COPY . .

# Build for production
ARG VITE_API_URL=http://localhost:3000
ARG VITE_CHAIN_ID=31337
ENV VITE_API_URL=$VITE_API_URL
ENV VITE_CHAIN_ID=$VITE_CHAIN_ID

RUN npm run build

# Stage 2: Production
FROM nginx:alpine AS production

# Copy nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy built assets
COPY --from=builder /app/dist /usr/share/nginx/html

# Add healthcheck
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
  CMD wget --quiet --tries=1 --spider http://localhost:80/ || exit 1

# Expose port
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
