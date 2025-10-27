FROM golang:1.24 AS builder
WORKDIR /app
COPY go.mod ./
COPY . .
RUN go get github.com/statsig-io/statsig-go-core@latest
RUN go get github.com/statsig-io/go-sdk@latest
RUN go mod download && go mod verify

RUN go build -o main .

FROM golang:1.24
WORKDIR /app
COPY --from=builder /app/main /app/main

CMD ["./main"]