FROM golang:1.24 AS builder
WORKDIR /app
COPY go.mod ./
COPY . .
RUN go get github.com/statsig-io/statsig-server-core/statsig-go@latest
RUN go get github.com/statsig-io/go-sdk@latest
RUN go mod download && go mod verify

RUN go install github.com/statsig-io/statsig-server-core/statsig-go/cmd/post-install@latest
RUN export PATH="$PATH:$(go env GOPATH)/bin" && post-install

ENV STATSIG_LIB_DIR="/root/.cache/statsig/resources"
ENV CGO_CFLAGS="-I$STATSIG_LIB_DIR"
ENV CGO_LDFLAGS="-L$STATSIG_LIB_DIR"
ENV LD_LIBRARY_PATH="$STATSIG_LIB_DIR"

RUN go build -o main .

FROM golang:1.24
WORKDIR /app
COPY --from=builder /app/main /app/main
COPY --from=builder /root/.cache/statsig/resources /root/.cache/statsig/resources

ENV STATSIG_LIB_DIR="/root/.cache/statsig/resources"
ENV LD_LIBRARY_PATH="$STATSIG_LIB_DIR"

CMD ["./main"]