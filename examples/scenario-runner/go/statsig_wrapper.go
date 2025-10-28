package main

import (
	"encoding/json"
	"os"

	statsig_legacy "github.com/statsig-io/go-sdk"
	statsig_core "github.com/statsig-io/statsig-go-core"
)

const SCRAPI_URL = "http://scrapi:8000"

type StatsigWrapper struct {
	isCore     bool
	core       *statsig_core.Statsig
	coreUser   *statsig_core.StatsigUser
	legacyUser *statsig_legacy.User
}

func Initialize() StatsigWrapper {
	variant := os.Getenv("SDK_VARIANT")
	isCore := variant == "core"

	if isCore {
		options, err := statsig_core.NewOptionsBuilder().
			WithSpecsUrl(SCRAPI_URL + "/v2/download_config_specs").
			WithLogEventUrl(SCRAPI_URL + "/v1/log_event").
			Build()

		if err != nil {
			panic(err)
		}

		core, err := statsig_core.NewStatsigWithOptions("secret-GO_CORE", options)
		if err != nil {
			panic(err)
		}

		core.Initialize()

		return StatsigWrapper{
			core:   core,
			isCore: true,
		}
	}

	options := statsig_legacy.Options{
		API: SCRAPI_URL + "/v1",
	}
	initDetails := statsig_legacy.InitializeWithOptions("secret-GO_LEGACY", &options)
	if !initDetails.Success {
		panic(initDetails.Error)
	}

	return StatsigWrapper{
		isCore: false,
	}
}

func (w *StatsigWrapper) SetUser(userID string) {
	if w.isCore {
		u, err := statsig_core.NewUserBuilderWithUserID(userID).Build()
		if err != nil {
			panic(err)
		}
		w.coreUser = u
	} else {
		u := statsig_legacy.User{
			UserID: userID,
		}
		w.legacyUser = &u
	}
}

func (w *StatsigWrapper) CheckGate(gateName string) bool {
	if w.isCore {
		return w.core.CheckGate(w.coreUser, gateName)
	}
	return statsig_legacy.CheckGate(*w.legacyUser, gateName)
}

func (w *StatsigWrapper) LogEvent(eventName string) {
	if w.isCore {
		w.core.LogEvent(w.coreUser, statsig_core.EventPayload{
			EventName: eventName,
		})
		return
	}

	statsig_legacy.LogEvent(statsig_legacy.Event{
		EventName: eventName,
		User:      *w.legacyUser,
	})
}

func (w *StatsigWrapper) GetClientInitializeResponse() string {
	if w.isCore {
		return *w.core.GetClientInitResponse(w.coreUser)
	}

	response := statsig_legacy.GetClientInitializeResponse(*w.legacyUser)
	json, err := json.Marshal(response)
	if err != nil {
		panic(err)
	}
	return string(json)
}
