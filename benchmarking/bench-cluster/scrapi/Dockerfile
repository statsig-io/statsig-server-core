FROM node:20 AS builder
WORKDIR /app
COPY . .
RUN npm install express tsx lodash

FROM node:20-slim
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY . .
RUN npm config set update-notifier false

CMD ["npx", "tsx", "index.mts"]

