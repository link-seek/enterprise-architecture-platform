FROM node:20-alpine AS builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm install
COPY . .
# Always use relative URLs in the Docker image. The nginx reverse proxy
# (see nginx.conf) forwards /api/ and /graphql to the backend container.
# This ensures E2E tests in CI reach the local backend instead of a
# hardcoded production host that may be baked in via build args.
ENV VITE_API_URL=/api
ENV VITE_GRAPHQL_URL=/graphql
RUN npx vite build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
