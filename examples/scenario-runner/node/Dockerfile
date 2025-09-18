FROM node:20 AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y jq
RUN npm install express tsx

ARG CACHE_BUST=$(date +%s)

RUN CORE_VERSION=$(npm view @statsig/statsig-node-core time --json | jq -r 'to_entries | map(select(.key | contains("-beta"))) | .[-1] | "\(.key)"') && \
    npm install @statsig/statsig-node-core@$CORE_VERSION

RUN LEGACY_VERSION=$(npm view statsig-node time --json | jq -r 'to_entries[-1] | "\(.key)"') && \
    npm install statsig-node@$LEGACY_VERSION

RUN IN=$CACHE_BUST npm list

FROM node:20-slim
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY . .
RUN npm config set update-notifier false

CMD ["npx", "tsx", "index.mts"]

