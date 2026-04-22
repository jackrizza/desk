FROM node:20-alpine AS development-dependencies-env

WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci

FROM node:20-alpine AS production-dependencies-env

WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev

FROM node:20-alpine AS build-env

ARG VITE_API_BASE_URL=/api

WORKDIR /app
COPY . .
COPY --from=development-dependencies-env /app/node_modules /app/node_modules
ENV VITE_API_BASE_URL=$VITE_API_BASE_URL
RUN npm run build

FROM node:20-alpine

WORKDIR /app
COPY package.json package-lock.json ./
COPY --from=production-dependencies-env /app/node_modules /app/node_modules
COPY --from=build-env /app/build /app/build

ENV HOST=0.0.0.0
ENV PORT=5173

EXPOSE 5173

CMD ["npx", "react-router-serve", "./build/server/index.js", "--host", "0.0.0.0", "--port", "5173"]
