package jwt

import (
	"encoding/base64"
	"encoding/json"
	"strings"
)

type Header struct {
	Algorithm string `json:"alg"`
	Type      string `json:"typ"`
}

func (h *Header) encode() (string, error) {
	data, err := json.Marshal(h)
	if err != nil {
		return "", err
	}

	return base64.RawURLEncoding.EncodeToString(data), nil
}

type Payload struct {
	Subject        string `json:"sub,omitempty"`
	ExpirationTime int64  `json:"exp,omitempty"`
	IssuedAt       int64  `json:"iat,omitempty"`
}

func (p *Payload) encode() (string, error) {
	data, err := json.Marshal(p)
	if err != nil {
		return "", err
	}

	return base64.RawURLEncoding.EncodeToString(data), nil
}

func Encode(jwtPayload *Payload) string {
	header, err := new(Header{
		Algorithm: "HS256",
		Type:      "JWT",
	}).encode()
	if err != nil {
		panic(err)
	}

	payload, err := jwtPayload.encode()
	if err != nil {
		panic(err)
	}

	return strings.Join([]string{header, payload, "mock"}, ".")
}
