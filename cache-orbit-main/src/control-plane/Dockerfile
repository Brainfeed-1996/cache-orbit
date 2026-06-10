FROM golang:1.23-bookworm AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /control-plane .

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=builder /control-plane /control-plane
EXPOSE 8080 50051
ENTRYPOINT ["/control-plane"]
CMD []
