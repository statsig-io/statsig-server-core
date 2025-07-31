FROM node:20 AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y jq
RUN npm install express tsx statsig-node
RUN CORE_VERSION=$(npm view @statsig/statsig-node-core time --json | jq -r 'to_entries[-1] | "\(.key)"') && \
    npm install @statsig/statsig-node-core@$CORE_VERSION


FROM node:20-slim
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY . .
RUN npm config set update-notifier false

CMD ["npx", "tsx", "index.mts"]

