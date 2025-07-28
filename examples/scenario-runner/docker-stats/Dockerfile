FROM alpine:latest

RUN apk add --no-cache docker-cli jq curl
COPY . . 

CMD ["sh", "main.sh"]