package main

import (
	"encoding/json"
	"flag"
	"log/slog"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
)

type Response struct {
	Code    int    `json:"code"`
	Message string `json:"msg"`
	Data    any    `json:"data,omitempty"`
	Total   *int   `json:"total,omitempty"`
}

func newResponse(data any) Response {
	return Response{Code: 0, Message: "success", Data: data}
}

func fail(code int, msg string) Response {
	return Response{Code: code, Message: msg}
}

func writeJSON(w http.ResponseWriter, data any) {
	w.Header().Set("Content-Type", "application/json")

	if err := json.NewEncoder(w).Encode(data); err != nil {
		slog.Warn("json encoding failed", "err", err)
	}
}

func emptyResponse(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, newResponse(nil))
}

func fixture(data []byte) http.HandlerFunc {
	return func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, newResponse(json.RawMessage(data)))
	}
}

func main() {
	addr := flag.String("addr", ":8080", "listen address")
	flag.Parse()

	r := chi.NewRouter()
	r.Use(middleware.Logger)

	registerUserRoutes(r)

	slog.Info("Listening", "addr", *addr)
	if err := http.ListenAndServe(*addr, r); err != nil {
		slog.Error("Server failed", "error", err)
		panic(err)
	}
}
