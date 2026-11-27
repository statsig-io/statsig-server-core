FROM node:22.23.4-bookworm@sha256:7702d8aa4d5910871692af4480eb3ba86f23047a6417a44d857d06c95d060d3f AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y jq
RUN npm install express tsx

# This next line prevent caching of the lines below it
ADD "https://www.random.org/cgi-bin/randbyte?nbytes=10&format=h" skipcache

RUN CORE_VERSION=$(npm view @statsig/statsig-node-core time --json | jq -r 'to_entries | map(select(.key | contains("-beta"))) | .[-1] | "\(.key)"') && \
    npm install @statsig/statsig-node-core@$CORE_VERSION

RUN LEGACY_VERSION=$(npm view statsig-node time --json | jq -r 'to_entries[-1] | "\(.key)"') && \
    npm install statsig-node@$LEGACY_VERSION

RUN npm list

FROM node:22.23.4-bookworm-slim@sha256:5ec23b7c622b00547753bc6ffd78085dd868a68aeb41c8e0a1418f033e13d96d
WORKDIR /app
COPY --from=builder /app/node_modules ./node_modules
COPY . .
RUN npm config set update-notifier false

CMD ["npx", "tsx", "index.mts"]
