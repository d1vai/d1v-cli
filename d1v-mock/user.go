package main

import (
	_ "embed"
	"encoding/json"
	"net/http"
	"time"

	"d1v-mock/jwt"

	"github.com/go-chi/chi/v5"
)

//go:embed fixtures/user.json
var userData []byte

//go:embed fixtures/user-admin.json
var userAdminData []byte

//go:embed fixtures/user-super-admin.json
var userSuperAdminData []byte

//go:embed fixtures/user-agent.json
var userAgentData []byte

//go:embed fixtures/activity.json
var activityData []byte

var defaultToken = jwt.Encode(&jwt.Payload{
	Subject:        "d1v",
	ExpirationTime: 9999999999,
	IssuedAt:       time.Now().Unix(),
})

var expiredToken = jwt.Encode(&jwt.Payload{
	Subject:        "d1v",
	ExpirationTime: 1000000000,
	IssuedAt:       1000000000,
})

func scenario(r *http.Request) string {
	return r.Header.Get("X-Test-Scenario")
}

const (
	scenarioFail       = "fail"
	scenarioExpired    = "expired"
	scenarioNoPassword = "nopassword"
	scenarioInvalid    = "invalid"
	scenarioAdmin      = "admin"
	scenarioSuperAdmin = "super-admin"
	scenarioAgent      = "agent"
)

func codeLogin(w http.ResponseWriter, r *http.Request) {
	switch scenario(r) {
	case scenarioExpired:
		writeJSON(w, newResponse(expiredToken))
	case scenarioFail:
		writeJSON(w, fail(1, "login failed"))
	default:
		writeJSON(w, newResponse(defaultToken))
	}
}

func passwordLogin(w http.ResponseWriter, r *http.Request) {
	switch scenario(r) {
	case scenarioExpired:
		writeJSON(w, newResponse(expiredToken))
	case scenarioFail:
		writeJSON(w, fail(1, "login failed"))
	case scenarioNoPassword:
		writeJSON(w, fail(40000, "password not set"))
	default:
		writeJSON(w, newResponse(defaultToken))
	}
}

func verifyCode(w http.ResponseWriter, r *http.Request) {
	switch scenario(r) {
	case scenarioFail:
		writeJSON(w, fail(1, "send code failed"))
	case scenarioInvalid:
		writeJSON(w, fail(2, "invalid email"))
	default:
		writeJSON(w, newResponse(nil))
	}
}

func checkCode(w http.ResponseWriter, r *http.Request) {
	if scenario(r) == scenarioInvalid {
		writeJSON(w, fail(3, "invalid verification code"))
		return
	}
	writeJSON(w, newResponse(nil))
}

func passwordHandler(w http.ResponseWriter, r *http.Request) {
	switch scenario(r) {
	case scenarioFail:
		writeJSON(w, fail(1, "operation failed"))
	case scenarioInvalid:
		writeJSON(w, fail(3, "invalid code"))
	default:
		writeJSON(w, newResponse(nil))
	}
}

func emailHandler(w http.ResponseWriter, r *http.Request) {
	switch scenario(r) {
	case scenarioFail:
		writeJSON(w, fail(1, "operation failed"))
	case scenarioInvalid:
		writeJSON(w, fail(3, "invalid code"))
	default:
		writeJSON(w, newResponse(nil))
	}
}

func userInfo(w http.ResponseWriter, r *http.Request) {
	switch scenario(r) {
	case scenarioAdmin:
		writeJSON(w, newResponse(json.RawMessage(userAdminData)))
	case scenarioSuperAdmin:
		writeJSON(w, newResponse(json.RawMessage(userSuperAdminData)))
	case scenarioAgent:
		writeJSON(w, newResponse(json.RawMessage(userAgentData)))
	default:
		writeJSON(w, newResponse(json.RawMessage(userData)))
	}
}

func allUsers(w http.ResponseWriter, _ *http.Request) {
	users := []json.RawMessage{
		json.RawMessage(userData),
		json.RawMessage(userAdminData),
		json.RawMessage(userSuperAdminData),
		json.RawMessage(userAgentData),
	}

	resp := newResponse(users)
	resp.Total = new(len(users))
	writeJSON(w, resp)
}

func registerUserRoutes(r chi.Router) {
	r.Route("/api/user", func(r chi.Router) {
		r.Post("/verify-code", verifyCode)
		r.Post("/verify-code/check", checkCode)
		r.Post("/login", codeLogin)
		r.Post("/login/password", passwordLogin)
		r.Post("/password/login", passwordLogin)

		r.Get("/info", userInfo)
		r.Put("/info", userInfo)
		r.Get("/public/{user_id}", userInfo)
		r.Get("/public/slug/{slug}", userInfo)
		r.Get("/all", allUsers)

		r.Post("/password/set", emptyResponse)
		r.Post("/password/forgot/send", passwordHandler)
		r.Post("/password/reset", passwordHandler)

		r.Post("/bind-email/send", emailHandler)
		r.Post("/bind-email/confirm", emailHandler)
		r.Post("/email/change/send", emailHandler)
		r.Post("/email/change/confirm", emailHandler)

		r.Post("/invitation/accept", emptyResponse)
		r.Get("/invitations", allUsers)

		r.Post("/onboarded/set", emptyResponse)
		r.Get("/activity/prompt-daily", fixture(activityData))
		r.Get("/activity/prompt-daily/slug/{slug}", fixture(activityData))
		r.Get("/activity/prompt-daily/user/{user_id}", fixture(activityData))
	})
}
