package main

import (
	_ "embed"
	"net/http"
	"time"

	"d1v-mock/jwt"

	"github.com/go-chi/chi/v5"
)

//go:embed fixtures/user.json
var userData []byte

//go:embed fixtures/activity.json
var activityData []byte

var defaultToken = jwt.Encode(&jwt.Payload{
	Subject:        "d1v",
	ExpirationTime: 9999999999,
	IssuedAt:       time.Now().Unix(),
})

func token(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, newResponse(defaultToken))
}

func registerUserRoutes(r chi.Router) {
	r.Route("/api/user", func(r chi.Router) {
		// Auth
		r.Post("/verify-code", emptyResponse)
		r.Post("/verify-code/check", emptyResponse)
		r.Post("/login", token)
		r.Post("/login/password", token)
		r.Post("/password/login", token)

		// User info
		r.Get("/info", fixture(userData))
		r.Put("/info", fixture(userData))
		r.Get("/public/{user_id}", fixture(userData))
		r.Get("/public/slug/{slug}", fixture(userData))
		r.Get("/all", fixtureList(userData))

		// Password
		r.Post("/password/set", emptyResponse)
		r.Post("/password/forgot/send", emptyResponse)
		r.Post("/password/reset", emptyResponse)

		// Email
		r.Post("/bind-email/send", emptyResponse)
		r.Post("/bind-email/confirm", emptyResponse)
		r.Post("/email/change/send", emptyResponse)
		r.Post("/email/change/confirm", emptyResponse)

		// Invitations
		r.Post("/invitation/accept", emptyResponse)
		r.Get("/invitations", fixtureList(userData))

		// Other
		r.Post("/onboarded/set", emptyResponse)
		r.Get("/activity/prompt-daily", fixture(activityData))
		r.Get("/activity/prompt-daily/slug/{slug}", fixture(activityData))
		r.Get("/activity/prompt-daily/user/{user_id}", fixture(activityData))
	})
}
