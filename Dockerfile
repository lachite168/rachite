# syntax=docker/dockerfile:1
FROM rust:1-trixie AS api-build
RUN apt-get update && apt-get install -y --no-install-recommends cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps/api apps/api
RUN cargo build --release --locked -p starter-api

FROM node:26.13.1-alpine@sha256:6bd9ab43c0ec0190ca7b2d193906cb8b7edf6e7f4afa43ddeccd6452710bffb5 AS web-build
WORKDIR /app
COPY package.json package-lock.json .npmrc .node-version ./
COPY apps/web/package.json apps/web/package.json
RUN --mount=type=cache,target=/root/.npm node -e 'const version = require("node:fs").readFileSync(".node-version", "utf8").trim(); if (process.versions.node !== version) throw new Error("Node.js must match .node-version: " + version)' && npm ci
COPY apps/web apps/web
RUN npm run build --workspace @starter/web

FROM debian:trixie-slim AS api
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* && useradd --system --uid 10001 --create-home app && mkdir /data && chown app:app /data
WORKDIR /app
COPY --from=api-build /app/target/release/starter-api /usr/local/bin/starter-api
ENV HOST=0.0.0.0 PORT=3000 DATABASE_URL=sqlite:///data/app.db
USER app
EXPOSE 3000
CMD ["starter-api"]

FROM nginx:stable-alpine AS web
COPY infra/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=web-build /app/apps/web/dist /usr/share/nginx/html
EXPOSE 80
