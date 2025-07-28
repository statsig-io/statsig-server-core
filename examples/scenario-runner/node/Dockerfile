FROM node:20 AS builder
WORKDIR /app
RUN npm install express tsx @statsig/statsig-node-core statsig-node

FROM node:20-slim
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY . .
RUN npm config set update-notifier false

CMD ["npx", "tsx", "index.mts"]

