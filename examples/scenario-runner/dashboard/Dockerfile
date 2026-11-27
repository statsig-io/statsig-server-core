FROM node:22.23.4-bookworm@sha256:7702d8aa4d5910871692af4480eb3ba86f23047a6417a44d857d06c95d060d3f AS builder
WORKDIR /app
RUN npm install express tsx

FROM node:22.23.4-bookworm-slim@sha256:5ec23b7c622b00547753bc6ffd78085dd868a68aeb41c8e0a1418f033e13d96d
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY . .
RUN npm config set update-notifier false

CMD ["npx", "tsx", "index.mts"]
